//! Executes the simulation, driving progress forward by repeatedly calling the engine's `step()` function.
//! Allows for things like rendering, using external data, etc. to be implemented between steps.

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
pub struct BasicDriver;

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Driver<C, E, M, CA, EA, N> for BasicDriver {
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
