// Taken from https://blog.fazibear.me/definitive-guide-to-rust-sdl-2-and-emscripten-93d707b22bbb
// Which took it from https://github.com/Gigoteur/PX8/blob/master/src/px8/emscripten.rs

use std::cell::RefCell;
use std::mem;
use std::ptr::{self, null_mut};
use std::os::raw::{c_int, c_void, c_float};

use prelude::*;
use container::EntityContainer;
use util::ColorCalculator;

#[allow(non_camel_case_types)]
type em_callback_func = unsafe extern fn();

extern {
    pub fn emscripten_set_main_loop(func: em_callback_func, fps: c_int, simulate_infinite_loop: c_int);
    pub fn emscripten_cancel_main_loop();
    pub fn emscripten_get_now() -> c_float;
}

thread_local!(static MAIN_LOOP_CALLBACK: RefCell<*mut c_void> = RefCell::new(null_mut()));

pub fn set_main_loop_callback<F>(callback: F) where F: FnMut() {
    MAIN_LOOP_CALLBACK.with(|log| {
        *log.borrow_mut() = &callback as *const _ as *mut c_void;
    });

    unsafe { emscripten_set_main_loop(wrapper::<F>, 0, 1); }

    unsafe extern "C" fn wrapper<F>() where F: FnMut() {
        MAIN_LOOP_CALLBACK.with(|z| {
            let closure = *z.borrow_mut() as *mut F;
            (*closure)();
        });
    }
}

/// Driver that integrates with the Emscripten browser event loop API to have the simulation loop automatically managed
/// by the browser.
pub struct EmscriptenDriver;

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, G: Engine<C, E, M, CA, EA>
> Driver<C, E, M, CA, EA, G> for EmscriptenDriver {
    fn init(
        self,
        mut universe: Universe<C, E, M, CA, EA>,
        mut engine: G,
        middleware: &mut [Box<Middleware<C, E, M, CA, EA, G>>]
    ) {
        let closure = || {
            for m in middleware.iter_mut() {
                m.before_render(&mut universe);
            }

            engine.step(&mut universe);

            for m in middleware.iter_mut() {
                m.after_render(&mut universe);
            }
        };

        // register the driver's core logic to be called by Emscripten automatically
        set_main_loop_callback(closure);
    }
}

/// Middleware that caculates the color of each pixel in the universe using a provided function and maintains
/// an internal buffer containing that data.  Once all of the data has been calculated, it calls the provided
/// `canvas_render` function with a pointer to that internal pixeldata buffer in rgba format (the same format
/// as is accepted by HTML Canvases).
pub struct CanvasRenderer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    pixbuf: Vec<u8>,
    get_color: ColorCalculator<C, E, M>,
    canvas_render: unsafe extern fn(ptr: *const u8),
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, G: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, G> for CanvasRenderer<C, E, M> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        // check if the universe size has changed since the last render and, if it has, re-size our pixbuf
        let universe_len = universe.cells.len();
        let expected_pixbuf_size = universe_len * 4;
        if expected_pixbuf_size != self.pixbuf.len() && expected_pixbuf_size != 0 {
            self.pixbuf.resize(expected_pixbuf_size, 255u8);
        }

        // update our internal pixel data buffer from the universe
        for universe_index in 0..universe.cells.len() {
            let entities = universe.entities.get_entities_at(universe_index);

            let dst_ptr = unsafe { self.pixbuf.as_ptr().offset(universe_index as isize * 4) } as *mut u32;
            unsafe {
                ptr::write(
                    dst_ptr,
                    mem::transmute::<[u8; 4], _>(
                        (self.get_color)(&universe.cells.get_unchecked(universe_index), entities, &universe.entities)
                    )
                )
            };
        }

        // pass a pointer to our internal buffer to the canvas render function
        unsafe { (self.canvas_render)(self.pixbuf.as_ptr()) }
    }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> CanvasRenderer<C, E, M> {
    pub fn new(
        universe_size: usize, get_color: ColorCalculator<C, E, M>, canvas_render: unsafe extern fn(ptr: *const u8)
    ) -> Self {
        CanvasRenderer {
            pixbuf: vec![255u8; universe_size * universe_size * 4],
            get_color,
            canvas_render,
        }
    }
}
