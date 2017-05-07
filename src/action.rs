//! Represents an entity's attempt to mutate the state of itself, another entity, or a cell.  These are returned
//! as the result of an entity's transformation function and are run through the universe's simulation engine and
//! applied according to the rules set up there.

use std::marker::PhantomData;

use entity::EntityState;
use cell::CellState;

pub struct Action<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>>  {
    pub x_offset: isize,
    pub y_offset: isize,
    pub action: TypedAction<C, E, CA, EA>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
}

#[allow(non_camel_case_types)]
pub enum TypedAction<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>>  {
    CellAction(CA),
    EntityAction(EA),
    SelfAction(SelfAction<C, E, EA>),
    __phantom_c(PhantomData<C>),
    __phantom_e(PhantomData<E>),
}

/// An attempt of an entity to mutate a cell.
pub trait CellAction<C: CellState> {}

/// An attempt of an entity to mutate another entity.
pub trait EntityAction<C: CellState, E: EntityState<C>> {}

/// An attempt of an entity to mutate itself.
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

impl<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> Action<C, E, CA, EA> {
    pub fn mut_self(self_action: SelfAction<C, E, EA>) -> Action<C, E, CA, EA> {
        Action {
            x_offset: 0,
            y_offset: 0,
            action: TypedAction::SelfAction(self_action),
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
        }
    }
}
