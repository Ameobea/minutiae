//! Executes the simulation, driving progress forward by repeatedly calling the engine's `step()` function.
//! Allows for things like rendering, using external data, etc. to be implemented between steps.

use action::{CellAction, EntityAction};
use cell::CellState;
use engine::Engine;
use entity::{EntityState, MutEntityState};
use universe::Universe;

pub mod middleware;
use self::middleware::Middleware;

pub trait Driver<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M>,
    N: Engine<C, E, M, CA, EA, U>,
>
{
    fn init(self, universe: U, N, Vec<Box<Middleware<C, E, M, CA, EA, U, N>>>);
}

/// Simplest implementation of a `Driver`.  Starts an infinite loop that steps the simulation's engine forever.
pub struct BasicDriver;

impl<
        C: CellState,
        E: EntityState<C>,
        M: MutEntityState,
        CA: CellAction<C>,
        EA: EntityAction<C, E>,
        U: Universe<C, E, M>,
        N: Engine<C, E, M, CA, EA, U>,
    > Driver<C, E, M, CA, EA, U, N> for BasicDriver
{
    fn init(self, mut universe: U, mut engine: N, mut middleware: Vec<Box<Middleware<C, E, M, CA, EA, U, N>>>) {
        println!("Starting simulation driver...");

        loop {
            for m in middleware.iter_mut() {
                m.before_render(&mut universe);
            }

            engine.step(&mut universe);

            for m in middleware.iter_mut() {
                m.after_render(&mut universe);
            }
        }
    }
}
