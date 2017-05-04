//! This is the core of the simulation.  It manages the various aspects of keeping track of the universe's
//! state and the state of all its cells and entities.  It drives the simulation forward by applying transformations
//! of the cells and processing actions of the entities sequentially.

use universe::Universe;
use cell::CellState;
use entity::EntityState;
use action::{Action, CellAction, EntityAction};

mod serial;
mod grid_iterator;

pub trait Engine<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>>{
    /// The main function of the simulation process.  This is called repeatedly to drive progress in the simulation and
    fn step(&mut self, &mut Universe<C, E, CA, EA, Self>) where Self:Sized;

    /// Given a requested `Action` from an Entity, checks for its validity against the engine's internal rules and
    /// applies it if it is successful.
    fn apply_actions(&mut self, &[Action<C, E, CA, EA>]);
}
