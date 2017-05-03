//! This is the core of the simulation.  It manages the various aspects of keeping track of the universe's
//! state and the state of all its cells and entities.  It drives the simulation forward by applying transformations
//! of the cells and processing actions of the entities sequentially.

use universe::Universe;
use cell::CellState;
use entity::EntityState;
use generator::Generator;

pub trait Engine<C: CellState, E: EntityState<C>>{
    /// The main function of the simulation process.  This is called repeatedly to drive progress in the simulation and
    fn step(&mut Universe<C, E, Self>) where Self:Sized;
}
