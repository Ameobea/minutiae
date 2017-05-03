//! Populates the world with an initial collection of cells and entities

use universe::Universe;
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use engine::Engine;

pub trait Generator<C: CellState, E: EntityState<C>, N: Engine<C, E>> {
    fn gen(&mut self, universe: &Universe<C, E, N>) -> (Vec<Cell<C>>, Vec<Vec<Entity<C, E>>>);
}
