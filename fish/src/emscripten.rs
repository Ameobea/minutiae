// Taken from https://blog.fazibear.me/definitive-guide-to-rust-sdl-2-and-emscripten-93d707b22bbb
// Which took it from https://github.com/Gigoteur/PX8/blob/master/src/px8/emscripten.rs

use std::cell::RefCell;
use std::mem;
use std::ptr::{self, null_mut};
use std::os::raw::{c_int, c_void, c_float};

use minutiae::driver::Driver;
use minutiae::driver::middleware::Middleware;
use super::*;

#[allow(non_camel_case_types)]
type em_callback_func = unsafe extern fn();

extern {
    pub fn emscripten_set_main_loop(func: em_callback_func, fps: c_int, simulate_infinite_loop: c_int);
    pub fn emscripten_cancel_main_loop();
    pub fn emscripten_get_now() -> c_float;

    pub fn canvas_render(ptr: *const u8);
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

type OurMiddlewareType = Middleware<
    OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, OurEngineType
>;

type OurUniverseType = Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>;

pub struct EmscriptenDriver;

impl Driver<
    OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, OurEngineType
> for EmscriptenDriver {
    fn init(
        self,
        mut universe: Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>,
        mut engine: OurEngineType,
        middleware: &mut [Box<OurMiddlewareType>]
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

pub struct CanvasRenderer(Box<[u8]>);

impl Middleware<
    OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, OurEngineType
> for CanvasRenderer {
    fn after_render(&mut self, universe: &mut OurUniverseType) {
        // update our internal pixel data buffer from the universe
        for universe_index in 0..universe.cells.len() {
            let entities = universe.entities.get_entities_at(universe_index);

            let dst_ptr = unsafe { self.0.as_ptr().offset(universe_index as isize * 4) } as *mut u32;
            if entities.len() > 0 {
                unsafe { ptr::write(dst_ptr, mem::transmute::<[u8; 4], _>([255, 233, 222, 255])) };
            } else {
                match unsafe { universe.cells.get_unchecked(universe_index) }.state {
                    OurCellState::Water => unsafe { ptr::write(dst_ptr, mem::transmute::<[u8; 4], _>([0, 0, 0, 255])) },
                    OurCellState::Food => unsafe { ptr::write(dst_ptr, mem::transmute::<[u8; 4], _>([13, 246, 24, 255])) },
                }
            };
        }

        // pass a pointer to our internal buffer to the canvas render function
        unsafe { canvas_render(self.0.as_ptr()) }
    }
}

impl CanvasRenderer {
    pub fn new() -> Self {
        CanvasRenderer(Box::new([255u8; UNIVERSE_SIZE * UNIVERSE_SIZE * 4]))
    }
}
