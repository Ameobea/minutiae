use std::ptr;

use wasm_bindgen::prelude::*;

use super::*;
use crate::prelude::*;

pub struct JSDriver {
    pub register_tick_callback: fn(closure: &Closure<(dyn std::ops::FnMut() -> ())>),
}

#[thread_local]
static mut CLOSURE: *mut Closure<(dyn std::ops::FnMut() + 'static)> = ptr::null_mut();

impl<
        C: CellState + 'static,
        E: EntityState<C> + 'static,
        M: MutEntityState + 'static,
        CA: CellAction<C> + 'static,
        EA: EntityAction<C, E> + 'static,
        U: Universe<C, E, M> + 'static,
        N: Engine<C, E, M, CA, EA, U> + 'static,
    > Driver<C, E, M, CA, EA, U, N> for JSDriver
{
    fn init(
        self,
        mut universe: U,
        mut engine: N,
        mut middleware: Vec<Box<dyn Middleware<C, E, M, CA, EA, U, N>>>,
    ) {
        // check to see if we have an existing closure (which in turn holds references to all of the
        // universe state) and drop it if we do.
        if unsafe { !CLOSURE.is_null() } {
            let closure = unsafe { Box::from_raw(CLOSURE) };
            drop(closure);
        }

        let cb = move || {
            for m in middleware.iter_mut() {
                m.before_render(&mut universe);
            }

            engine.step(&mut universe);

            for m in middleware.iter_mut() {
                m.after_render(&mut universe);
            }
        };

        let closure = Box::new(Closure::wrap((Box::new(cb)) as Box<dyn FnMut()>));
        (self.register_tick_callback)(&*closure);
        // hold onto the closure we created
        unsafe { CLOSURE = Box::into_raw(closure) };
    }
}
