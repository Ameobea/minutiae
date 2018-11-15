// Taken from https://blog.fazibear.me/definitive-guide-to-rust-sdl-2-and-emscripten-93d707b22bbb
// Which took it from https://github.com/Gigoteur/PX8/blob/master/src/px8/emscripten.rs

use std::cell::RefCell;
use std::mem;
use std::os::raw::{c_float, c_int, c_void};
use std::ptr::{self, null_mut};

use prelude::*;
use universe::Universe2D;
use util::ColorCalculator;

#[allow(non_camel_case_types)]
type em_callback_func = unsafe extern "C" fn();

extern "C" {
    pub fn emscripten_set_main_loop(func: em_callback_func, fps: c_int, simulate_infinite_loop: c_int);
    pub fn emscripten_cancel_main_loop();
    pub fn emscripten_get_now() -> c_float;
}

thread_local!(static MAIN_LOOP_CALLBACK: RefCell<*mut c_void> = RefCell::new(null_mut()));

pub fn set_main_loop_callback<F>(callback: F)
where
    F: FnMut(),
{
    MAIN_LOOP_CALLBACK.with(|log| {
        *log.borrow_mut() = &callback as *const _ as *mut c_void;
    });

    unsafe {
        emscripten_set_main_loop(wrapper::<F>, 0, 1);
    }

    unsafe extern "C" fn wrapper<F>()
    where
        F: FnMut(),
    {
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
        C: CellState,
        E: EntityState<C>,
        M: MutEntityState,
        CA: CellAction<C>,
        EA: EntityAction<C, E>,
        U: Universe<C, E, M>,
        N: Engine<C, E, M, CA, EA, U>,
    > Driver<C, E, M, CA, EA, U, N> for EmscriptenDriver
{
    fn init(self, mut universe: U, mut engine: N, mut middleware: Vec<Box<Middleware<C, E, M, CA, EA, U, N>>>) {
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
pub struct CanvasRenderer<C: CellState, E: EntityState<C>, M: MutEntityState, I: Ord + Copy> {
    pixbuf: Vec<u8>,
    get_color: ColorCalculator<C, E, M, I>,
    canvas_render: fn(colors: &[u8]),
}

impl<
        C: CellState,
        E: EntityState<C>,
        M: MutEntityState,
        CA: CellAction<C>,
        EA: EntityAction<C, E>,
        N: Engine<C, E, M, CA, EA, Universe2D<C, E, M>>,
    > Middleware<C, E, M, CA, EA, Universe2D<C, E, M>, N> for CanvasRenderer<C, E, M, usize>
{
    fn after_render(&mut self, universe: &mut Universe2D<C, E, M>) {
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
                    mem::transmute::<[u8; 4], _>((self.get_color)(
                        &universe.cells.get_unchecked(universe_index),
                        entities,
                        &universe.entities,
                    )),
                )
            };
        }

        // pass a pointer to our internal buffer to the canvas render function
        unsafe { (self.canvas_render)(&self.pixbuf) }
    }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState, I: Ord + Copy> CanvasRenderer<C, E, M, I> {
    pub fn new(universe_size: usize, get_color: ColorCalculator<C, E, M, I>, canvas_render: fn(colors: &[u8])) -> Self {
        CanvasRenderer {
            pixbuf: vec![255u8; universe_size * universe_size * 4],
            get_color,
            canvas_render,
        }
    }
}

pub enum KeyPress {
    Character(char),
    Enter,
    Other(u8),
}

/// Represents an action that the user can take in the web UI.
pub enum UserEvent {
    CanvasClick { x: u32, y: u32 },
    KeyPress,
    Custom(Vec<u8>),
}
