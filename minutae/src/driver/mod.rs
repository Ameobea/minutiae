//! Executes the simulation, driving progress forward by repeatedly calling the engine's `step()` function.
//! Allows for things like rendering, using external data, etc. to be implemented between steps.

use std::marker::PhantomData;

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use engine::Engine;

pub mod middleware;
use self::middleware::Middleware;

pub trait Driver<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> {
    fn init(self, universe: Universe<C, E, M, CA, EA>, N, &mut [Box<Middleware<C, E, M, CA, EA, N>>]);
}

/// Simplest implementation of a `Driver`.  Starts an infinite loop that steps the simulation's engine forever.
pub struct BasicDriver<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> {
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_n: PhantomData<N>,
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> BasicDriver<C, E, M, CA, EA, N> {
    pub fn new() -> BasicDriver<C, E, M, CA, EA, N> {
        BasicDriver {
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_n: PhantomData,
            __phantom_m: PhantomData,
        }
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Driver<C, E, M, CA, EA, N> for BasicDriver<C, E, M, CA, EA, N> {
    fn init(self, mut universe: Universe<C, E, M, CA, EA>, mut engine: N, middleware: &mut [Box<Middleware<C, E, M, CA, EA, N>>]) {
        println!("Starting simulation driver...");
        loop {
            for mut m in middleware.iter_mut() {
                m.before_render(&mut universe);
            }

            engine.step(&mut universe);

            for m in middleware.iter_mut() {
                m.after_render(&mut universe);
            }
        }
    }
}
