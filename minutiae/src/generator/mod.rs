//! Populates the world with an initial collection of cells and entities

use universe::UniverseConf;
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};

pub trait Generator<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<C>>, Vec<Vec<Entity<C, E, M>>>);
}
