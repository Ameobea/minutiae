//! These units reside in the a grid separate from the main universe but have the power to directly interact both with it and
//! with other entities.  They are the smallest units of discrete control in the simulation and are the only things that are
//! capable of mutating any aspect of the world outside of the simulation engine itself.

use std::marker::PhantomData;

use universe::Universe;
use action::Action;
use cell::{Cell, CellState};

pub trait EntityState<C: CellState> {
    fn transform(
        &self, &universe: &[Vec<Entity<C, Self>>], neighbor_entity_coords: &[usize], neighbor_cells: &[&Cell<C>]
    ) -> Action<C, Self> where Self:Sized;
}

pub struct Entity<C: CellState, S: EntityState<C>> {
    pub state: S,
    phantom: PhantomData<C>,
}
