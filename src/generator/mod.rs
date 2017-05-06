//! Populates the world with an initial collection of cells and entities

use universe::UniverseConf;
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use action::{CellAction, EntityAction};
use engine::Engine;

pub trait Generator<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, CA, EA>> {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<C>>, Vec<Vec<Entity<C, E>>>);
}
