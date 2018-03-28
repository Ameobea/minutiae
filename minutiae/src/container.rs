//! Declares container types that are used to provide abstracted access to data strucures within a universe.

use std::collections::BTreeMap;
use std::mem;
use std::ops::{Index, IndexMut};
use std::usize;

#[cfg(feature = "serde")]
use serde::Deserialize;

use uuid::Uuid;

use cell::CellState;
use entity::{Entity, EntityState, MutEntityState};

/// For each coordinate on the grid, keeps track of the entities that inhabit it by holding a list of
/// indexes to slots in the `EntityContainer`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EntityPositions<I: Ord>(pub BTreeMap<I, Vec<usize>>);

impl<I: Ord> EntityPositions<I> {
    pub fn new() -> Self {
        EntityPositions(BTreeMap::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

lazy_static! {
    static ref EMPTY: Vec<usize> = Vec::new();
}

impl<I: Ord> Index<I> for EntityPositions<I> {
    type Output = Vec<usize>;

    fn index<'a>(&'a self, index: I) -> &'a Self::Output {
        &self.0.get(&index).unwrap_or(&EMPTY) // I'm a legit genius
    }
}

impl<I: Ord> IndexMut<I> for EntityPositions<I> {
    fn index_mut<'a>(&'a mut self, index: I) -> &'a mut Self::Output {
        self.0.entry(index).or_insert(Vec::new())
    }
}

/// Either holds an entity or a 'pointer' (in the form of an array index) of the next empty slot in the data structure.
/// This functions somewhat similarly to a linked list.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound = "C: for<'d> Deserialize<'d>, I: ::serde::Serialize + for<'d> Deserialize<'d>"))]
pub enum EntitySlot<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    I,
> {
    Occupied{
        entity: Entity<C, E, M>,
        universe_index: I
    },
    Empty(usize),
}

unsafe impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    I,
> Send for EntitySlot<C, E, M, I> where C:Send, E:Send, M:Send, I: Send {}

/// Data structure holding all of the universe's entities.  The entities and their state are held in a vector of
/// `EntitySlot`s, each of which either holds an entity or the index of the next empty slot.  Using this method, it's
/// possible to add/remove entities from anywhere in the container without causing any allocations.
///
/// A second internal structure is used to map universe indexes to entity indexes; it holds the entity indexes of all
/// entities that reside in each universe index.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(bound = "C: for<'d> Deserialize<'d>, I: ::serde::Serialize + for<'d> Deserialize<'d>"))]
pub struct EntityContainer<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    I: Ord + Copy,
> {
    pub entities: Vec<EntitySlot<C, E, M, I>>,
    pub empty_index: usize,
    pub positions: EntityPositions<I>,
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    I: Ord + Copy,
> EntityContainer<C, E, M, I> {
    pub fn new() -> Self {
        // a point of `usize::MAX` indicates that the slot is the last available one.
        EntityContainer{
            entities: vec![EntitySlot::Empty(usize::MAX)],
            empty_index: 0,
            positions: EntityPositions::new()
        }
    }

    /// Inserts an entity into the container, returning its index
    pub fn insert(&mut self, entity: Entity<C, E, M>, universe_index: I) -> usize {
        let &mut EntityContainer{ref mut entities, empty_index, ref mut positions} = self;
        let entity_index = if empty_index != usize::MAX {
            let next_empty = match entities[empty_index] {
                EntitySlot::Empty(next_empty) => next_empty,
                _ => unreachable!(),
            };

            // update the index of the next empty cell and insert the entity into the empty slot
            self.empty_index = next_empty;
            entities[empty_index] = EntitySlot::Occupied{entity, universe_index};

            empty_index
        } else {
            // this is the last empty slot in the container, so we have to allocate space for another
            let entity_count = entities.len();
            entities.push(EntitySlot::Occupied{entity, universe_index});

            entity_count
        };

        // update the positions vector to store the index of the entity
        positions[universe_index].push(entity_index);

        entity_index
    }

    /// Removes an entity from the container, returning it along with its current index in the universe.
    pub fn remove(&mut self, entity_index: usize) -> Entity<C, E, M> {
        let removed = mem::replace(&mut self.entities[entity_index], EntitySlot::Empty(self.empty_index));
        self.empty_index = entity_index;

        match removed {
            EntitySlot::Occupied{entity, universe_index} => {
                // find the index of the index pointer inside the location vector and remove it
                let position_index = self.positions[universe_index]
                    .iter()
                    .position(|&index| index == entity_index)
                    .expect("Unable to locate entity index at the expected location in the positions vector!");
                let removed_entity_index = self.positions[universe_index].remove(position_index);
                debug_assert_eq!(removed_entity_index, entity_index);

                entity
            },
            EntitySlot::Empty(_) => unreachable!(),
        }
    }

    /// Returns a reference to the entity contained at the supplied index.  Will cause undefined behavior in
    /// release mode and panic in debug mode ifthe index is out of bounds or the slot at the specified index is empty.
    pub unsafe fn get(&self, index: usize) -> &Entity<C, E, M> {
        debug_assert!(index < self.entities.len());
        match self.entities.get_unchecked(index) {
            &EntitySlot::Occupied{ref entity, universe_index: _} => entity,
            _ => unreachable!(),
        }
    }

    /// Returns a mutable reference to the entity contained at the supplied index.  Will cause undefined behavior in
    /// release mode and panic in debug mode ifthe index is out of bounds or the slot at the specified index is empty.
    pub unsafe fn get_mut(&mut self, index: usize) -> &mut Entity<C, E, M> {
        debug_assert!(index < self.entities.len());
        match self.entities.get_unchecked_mut(index) {
            &mut EntitySlot::Occupied{ref mut entity, universe_index: _} => entity,
            _ => unreachable!(),
        }
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the supplied UUID.  If so, returns
    /// a reference to the contained entity and its corresponding universe index.
    pub fn get_verify(&self, index: usize, uuid: Uuid) -> Option<(&Entity<C, E, M>, I)> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked(index) } {
            &EntitySlot::Occupied { ref entity, ref universe_index } => {
                if entity.uuid == uuid { Some((entity, *universe_index)) } else { None }
            },
            _ => None,
        }
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the supplied UUID.  If so, returns
    /// the a mutable reference to the contained entity and its corresponding universe index.
    pub fn get_verify_mut(&mut self, index: usize, uuid: Uuid) -> Option<(&mut Entity<C, E, M>, I)> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked_mut(index) } {
            &mut EntitySlot::Occupied{ref mut entity, ref universe_index} => {
                if entity.uuid == uuid { Some((entity, *universe_index)) } else { None }
            },
            _ => None,
        }
    }

    /// Moves an entity from one location in the universe to another.  This function assumes that the supplied index
    /// is occupied and that the destination index is sane.
    pub fn move_entity(&mut self, entity_index: usize, dst_universe_index: I) {
        debug_assert!(entity_index < self.entities.len());
        let src_universe_index: I = match self.entities[entity_index] {
            EntitySlot::Occupied{entity: _, ref mut universe_index} => {
                // update the universe index within the entity slot
                let src_universe_index: I = *universe_index;
                *universe_index = dst_universe_index;

                src_universe_index
            },
            _ => unreachable!(),
        };

        // remove the index of the entity from the old universe index
        let position_index = self.positions[src_universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect("Unable to locate entity index at the expected location in the positions vector!");
        let removed = self.positions[src_universe_index].remove(position_index);
        debug_assert_eq!(entity_index, removed);
        // and add it to the new universe index in the position vector
        self.positions[dst_universe_index].push(entity_index);
    }

    /// Creates an iterator over the entities contained within the container with the format
    /// `(Entity, entity_index, universe_index)`.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(&'a Entity<C, E, M>, usize, I)> {
        self.entities.iter()
            .enumerate()
            .filter(|&(_, slot)| match slot {
                &EntitySlot::Occupied{entity: _, universe_index: _} => true,
                &EntitySlot::Empty(_) => false,
            }).map(|(entity_index, slot)| match slot {
                &EntitySlot::Occupied { ref entity, ref universe_index } => (entity, entity_index, *universe_index),
                _ => unreachable!(),
            })
    }

    /// Creates a mutable iterator over the entities contained within the container with the format
    /// `(Entity, entity_index, universe_index)`.
    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=(&'a mut Entity<C, E, M>, usize, I)> {
        self.entities.iter_mut()
            .enumerate()
            .filter(|&(_, ref slot)| match slot {
                &&mut EntitySlot::Occupied { entity: _, universe_index: _ } => true,
                &&mut EntitySlot::Empty(_) => false,
            }).map(|(entity_index, slot)| match slot {
                &mut EntitySlot::Occupied { ref mut entity, ref universe_index} => (entity, entity_index, *universe_index),
                _ => unreachable!(),
            })
    }

    /// Returns the position of the entity with the given entity index.
    pub fn get_position_index(&self, entity_index: usize) -> usize {
        debug_assert!(self.entities.len() > entity_index);
        let universe_index = match self.entities[entity_index] {
            EntitySlot::Occupied{entity: _, universe_index} => universe_index,
            _ => unreachable!(),
        };

        self.positions[universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect("Unable to find entry in position vector at at index pointed to by entity vector!")
    }

    /// Returns a reference to the slice of all the `entity_index`es of all entities at a certain universe index.
    pub fn get_entities_at(&self, universe_index: I) -> &[usize] {
        &self.positions[universe_index]
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
}
