//! These units reside in the a grid separate from the main universe but have the power to directly interact both with it and
//! with other entities.  They are the smallest units of discrete control in the simulation and are the only things that are
//! capable of mutating any aspect of the world outside of the simulation engine itself.

use std::marker::PhantomData;
use std::clone::Clone;

use cell::CellState;

pub trait EntityState<C: CellState> {}

#[derive(Debug)]
pub struct Entity<C: CellState, S: EntityState<C>> {
    pub state: S,
    phantom: PhantomData<C>,
}

impl<C: CellState, E: EntityState<C>> Clone for Entity<C, E> where E:Clone {
    fn clone(&self) -> Self {
        Entity {
            state: self.state.clone(),
            phantom: PhantomData,
        }
    }
}
