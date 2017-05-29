//! These units reside in the a grid separate from the main universe but have the power to directly interact both with it and
//! with other entities.  They are the smallest units of discrete control in the simulation and are the only things that are
//! capable of mutating any aspect of the world outside of the simulation engine itself.

use std::cell::Cell as RustCell;
use std::clone::Clone;
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[allow(unused_imports)]
use test;

use cell::CellState;

/// The core state of an entity that defines its behavior.  This stat is modified by the engine and is visible by not
/// writable by the entity itself.
#[cfg(feature = "serde")]
pub trait EntityState<C: CellState>:Clone + Serialize + for<'d> Deserialize<'d> {}

#[cfg(not(feature = "serde"))]
pub trait EntityState<C: CellState>:Clone {}

/// Entity state that is private to the entity.  It is not visible to other entities or to the engine but is mutable
/// during the entity driver and can be used to hold things such as PRNGs etc.
#[cfg(feature = "serde")]
pub trait MutEntityState:Clone + Copy + Default + Serialize + for<'d> Deserialize<'d> {}

#[cfg(not(feature = "serde"))]
pub trait MutEntityState:Clone + Default {}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound = "C: for<'d> Deserialize<'d>"))]
pub struct Entity<C: CellState, S: EntityState<C>, M: MutEntityState> {
    pub state: S,
    pub mut_state: RustCell<M>,
    pub uuid: Uuid,
    phantom: PhantomData<C>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Debug for Entity<C, E, M> where E:Debug {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "Entity {{state: {:?} }}", self.state)
    }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Clone for Entity<C, E, M> where E:Clone, M:Clone {
    fn clone(&self) -> Self {
        let mut_state_inner = self.mut_state.take();
        let mut_state_inner_clone = mut_state_inner.clone();
        self.mut_state.set(mut_state_inner_clone);
        Entity {
            state: self.state.clone(),
            mut_state: RustCell::new(mut_state_inner),
            uuid: Uuid::new_v4(),
            phantom: PhantomData,
        }
    }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Entity<C, E, M> {
    pub fn new(state: E, mut_state: M) -> Entity<C, E, M> {
        Entity {
            state: state,
            mut_state: RustCell::new(mut_state),
            uuid: Uuid::new_v4(),
            phantom: PhantomData,
        }
    }
}

unsafe impl<C: CellState, E: EntityState<C>, M: MutEntityState> Send for Entity<C, E, M> where C:Send, E:Send {}
