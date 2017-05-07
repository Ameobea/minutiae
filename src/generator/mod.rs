//! Populates the world with an initial collection of cells and entities

use universe::UniverseConf;
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use action::{CellAction, EntityAction};

pub trait Generator<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<C>>, Vec<Vec<Entity<C, E>>>);
}
