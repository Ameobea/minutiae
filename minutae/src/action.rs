//! Represents an entity's attempt to mutate the state of itself, another entity, or a cell.  These are returned
//! as the result of an entity's transformation function and are run through the universe's simulation engine and
//! applied according to the rules set up there.

use std::marker::PhantomData;

use uuid::Uuid;

use entity::EntityState;
use cell::CellState;

/// An action that is associated with a particular entity
#[derive(Debug)]
pub struct OwnedAction<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> {
    pub source_universe_index: usize,
    pub source_entity_index: usize,
    pub source_uuid: Uuid,
    pub action: Action<C, E, CA, EA>,
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Action<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>>  {
    CellAction {
        action: CA,
        x_offset: isize,
        y_offset: isize,
    },
    EntityAction {
        action: EA,
        x_offset: isize,
        y_offset: isize,
        target_uuid: Uuid,
    },
    SelfAction(SelfAction<C, E, EA>),
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
}

/// An attempt of an entity to mutate a cell.
pub trait CellAction<C: CellState> {}

/// An attempt of an entity to mutate another entity.
pub trait EntityAction<C: CellState, E: EntityState<C>> {}

/// An attempt of an entity to mutate itself.
#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum SelfAction<C: CellState, E: EntityState<C>, EA: EntityAction<C, E>> {
    Translate(isize, isize),
    Suicide,
    Custom(EA),
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
}

impl<C: CellState, E: EntityState<C>, EA: EntityAction<C, E>> SelfAction<C, E, EA> {
    pub fn translate(x: isize, y: isize) -> SelfAction<C, E, EA> {
        SelfAction::Translate(x, y)
    }
}
