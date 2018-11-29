//! Represents an entity's attempt to mutate the state of itself, another entity, or a cell.  These
//! are returned as the result of an entity's transformation function and are run through the
//! universe's simulation engine and applied according to the rules set up there.

use std::marker::PhantomData;

use uuid::Uuid;

use cell::CellState;
use entity::EntityState;

/// An action that is associated with a particular entity
#[derive(Debug)]
pub struct OwnedAction<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> {
    pub source_entity_index: usize,
    pub source_uuid: Uuid,
    pub action: Action<C, E, CA, EA>,
}

impl<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> Clone
    for OwnedAction<C, E, CA, EA>
where
    Action<C, E, CA, EA>: Clone,
{
    fn clone(&self) -> Self {
        OwnedAction {
            source_entity_index: self.source_entity_index,
            source_uuid: self.source_uuid,
            action: self.action.clone(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Action<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> {
    CellAction {
        action: CA,
        universe_index: usize,
    },
    EntityAction {
        action: EA,
        target_entity_index: usize,
        target_uuid: Uuid,
    },
    SelfAction(SelfAction<C, E, EA>),
}

impl<
        C: CellState + Clone,
        E: EntityState<C> + Clone,
        CA: CellAction<C> + Clone,
        EA: EntityAction<C, E> + Clone,
    > Clone for Action<C, E, CA, EA>
where
    SelfAction<C, E, EA>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            &Action::CellAction {
                ref action,
                ref universe_index,
            } => Action::CellAction {
                action: action.clone(),
                universe_index: *universe_index,
            },
            &Action::EntityAction {
                ref action,
                target_entity_index,
                target_uuid,
            } => Action::EntityAction {
                action: action.clone(),
                target_entity_index,
                target_uuid,
            },
            &Action::SelfAction(ref action) => Action::SelfAction(action.clone()),
        }
    }
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

impl<C: CellState, E: EntityState<C>, EA: EntityAction<C, E>> Clone for SelfAction<C, E, EA>
where
    EA: Clone,
{
    fn clone(&self) -> Self {
        match self {
            &SelfAction::Translate(x, y) => SelfAction::Translate(x, y),
            &SelfAction::Suicide => SelfAction::Suicide,
            &SelfAction::Custom(ref ea) => SelfAction::Custom(ea.clone()),
            &SelfAction::__phantom_c(spooky) => SelfAction::__phantom_c(spooky),
            &SelfAction::__phantom_e(scary) => SelfAction::__phantom_e(scary),
        }
    }
}

impl<C: CellState, E: EntityState<C>, EA: EntityAction<C, E>> SelfAction<C, E, EA> {
    pub fn translate(x: isize, y: isize) -> SelfAction<C, E, EA> { SelfAction::Translate(x, y) }
}
