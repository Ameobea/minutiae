//! These units reside in the a grid separate from the main universe but have the power to directly interact both with it and
//! with other entities.  They are the smallest units of discrete control in the simulation and are the only things that are
//! capable of mutating any aspect of the world outside of the simulation engine itself.

use std::marker::PhantomData;

use action::{Action, CellAction, EntityAction};
use cell::{Cell, CellState};

pub trait EntityState<C: CellState> {}

#[derive(Debug)]
pub struct Entity<C: CellState, S: EntityState<C>> {
    pub state: S,
    phantom: PhantomData<C>,
}

impl<C: CellState, S: EntityState<C>> Entity<C, S> {
    // fn transform<'a, CA: CellAction<C>, EA: EntityAction<C, Self>>(
    //     &self,
    //     entity_accessor: &Fn(isize, isize) -> Option<&'a Vec<Entity<C, Self>>>,
    //     cell_accessor: &Fn(isize, isize) -> Option<&'a Cell<C>>,
    //     action_executor: &FnMut(Action<C, Self, CA, EA>),
    // ) where Self:Sized;
}
