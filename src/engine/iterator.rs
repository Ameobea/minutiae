//! Defines an object that iterates over of a universe in some order.

use std::marker::PhantomData;

use cell::CellState;
use entity::{Entity, EntityState};

/// Visits the cells of a universe in a particular order returning the indexes of the cells it visits.
pub trait GridIterator {
    fn visit(&mut self) -> Option<usize>;
}

impl<'a> Iterator for &'a mut GridIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.visit()
    }
}

/// Iterates over the size of the universe
pub struct SerialGridIterator {
    pub universe_length: usize,
    pub index: usize,
    pub done: bool,
}

impl SerialGridIterator {
    pub fn new(universe_size: usize) -> SerialGridIterator {
        SerialGridIterator {
            universe_length: universe_size * universe_size,
            index: 0,
            done: false,
        }
    }
}

impl GridIterator for SerialGridIterator {
    fn visit(&mut self) -> Option<usize> {
        if self.done {
            self.done = false;
            self.index = 0;
        } else {
            self.index += 1;
        }

        if self.index <= self.universe_length {
            Some(self.index)
        } else {
            self.done = true;
            None
        }
    }
}

/// Visits the entities of a universe in a particular order, returning the index of the cell the entity inhabits
/// as well as the index of the entity within that cell (since there can be multiple entities in one cell).
pub trait EntityIterator<C: CellState, E: EntityState<C>> {
    fn visit(&mut self, entity_counts: &[Vec<Entity<C, E>>]) -> Option<(usize, usize)>;
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
            done: false,
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
        }
    }
}

fn access_entity<C: CellState, E: EntityState<C>>(counts: &[Vec<Entity<C, E>>], universe_index: usize, entity_index: usize) -> bool {
    if counts.len() < universe_index && counts[universe_index].len() < entity_index {
        true
    } else {
        false
    }
}

impl<C: CellState, E: EntityState<C>> EntityIterator<C, E> for SerialEntityIterator<C, E> {
    fn visit(&mut self, entity_counts: &[Vec<Entity<C, E>>]) -> Option<(usize, usize)> {
        if self.done {
            self.done = false;
            self.universe_index = 0;
            self.entity_index = 0;

            if access_entity(entity_counts, self.universe_index, 0) {
                return Some((0, 0));
            }
        } else {
            if access_entity(entity_counts, self.universe_index, self.entity_index + 1) {
                self.entity_index += 1;
                return Some((self.universe_index, self.entity_index))
            } else {
                self.entity_index = 0;
            }
        }

        // iterate over the remaining indexes of the universe and return the coordinates of the first found entity
        while self.entity_index < self.universe_length {
            self.entity_index += 1;
            if access_entity(entity_counts, self.universe_index, 0) {
                return Some((self.entity_index, 0))
            }
        }

        // if we finished the loop, then we've visited all entities in the universe.
        None
    }
}
