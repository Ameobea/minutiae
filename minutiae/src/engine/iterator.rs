//! Defines an object that iterates over of a universe in some order.

use std::collections::HashSet;
use std::marker::PhantomData;

use cell::CellState;
use entity::{Entity, EntityState, MutEntityState};

/// Visits the entities of a universe in a particular order, returning the index of the cell the entity inhabits
/// as well as the index of the entity within that cell (since there can be multiple entities in one cell).
pub trait EntityIterator<C: CellState, E: EntityState<C>, M: MutEntityState> {
    fn visit(&mut self, entities: &[Vec<Entity<C, E, M>>], entity_meta: &HashSet<usize>) -> Option<(usize, usize)>;
}

pub struct SerialEntityIterator<C: CellState, E: EntityState<C>> {
    pub universe_length: usize,
    pub universe_index: usize,
    pub entity_index: usize,
    pub done: bool,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
}

impl<C: CellState, E: EntityState<C>> SerialEntityIterator<C, E> {
    pub fn new(universe_length: usize) -> SerialEntityIterator<C, E> {
        SerialEntityIterator {
            universe_length: universe_length,
            universe_index: 0,
            entity_index: 0,
            done: true,
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
        }
    }
}

#[inline(always)]
fn access_entity<C: CellState, E: EntityState<C>, M: MutEntityState>(
    entity_meta: &HashSet<usize>, entities: &[Vec<Entity<C, E, M>>], universe_index: usize, entity_index: usize
) -> bool {
    entity_meta.contains(&universe_index) && entities[universe_index].len() > entity_index
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> EntityIterator<C, E, M> for SerialEntityIterator<C, E> {
    fn visit(&mut self, entities: &[Vec<Entity<C, E, M>>], entity_meta: &HashSet<usize>) -> Option<(usize, usize)> {
        if self.done {
            self.done = false;
            self.universe_index = 0;
            self.entity_index = 0;

            if access_entity(entity_meta, entities, 0, 0) {
                return Some((0, 0));
            }
        } else {
            if access_entity(entity_meta, entities, self.universe_index, self.entity_index + 1) {
                self.entity_index += 1;
                return Some((self.universe_index, self.entity_index))
            } else {
                self.entity_index = 0;
            }
        }

        // iterate over the remaining indexes of the universe and return the coordinates of the first found entity
        while self.universe_index < self.universe_length {
            self.universe_index += 1;
            if access_entity(entity_meta, entities, self.universe_index, 0) {
                return Some((self.universe_index, 0))
            }
        }

        // if we finished the loop, then we've visited all entities in the universe.
        None
    }
}
