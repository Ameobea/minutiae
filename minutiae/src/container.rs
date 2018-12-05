//! Declares container types that are used to provide abstracted access to data strucures within a
//! universe.

#[cfg(feature = "serde")]
use serde::Deserialize;

use slab::Slab;
use uuid::Uuid;

use cell::CellState;
use entity::{Entity, EntityState, MutEntityState};

/// Data structure holding all of the universe's entities.  The entities and their state are held in
/// a vector of `EntitySlot`s, each of which either holds an entity or the index of the next empty
/// slot.  Using this method, it's possible to add/remove entities from anywhere in the container
/// without causing any allocations.
///
/// A second internal structure is used to map universe indexes to entity indexes; it holds the
/// entity indexes of all entities that reside in each universe index.
#[derive(Clone, Debug)]
// TODO: https://github.com/carllerche/slab/pull/41/files
// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// #[cfg_attr(feature = "serde", serde(bound = "C: for<'d> Deserialize<'d>"))]
pub struct EntityContainer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    /// `(entity, universe_ix)`
    pub entities: Slab<(Entity<C, E, M>, usize)>,
    /// A mapping of universe index to entity index
    pub positions: Vec<Vec<usize>>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> EntityContainer<C, E, M> {
    pub fn new(universe_size: usize) -> Self {
        // a point of `u32::MAX` indicates that the slot is the last available one.
        // (We use `u32` instead of `usize` so that we can handle binary
        // serialization/deserialization on 32-bit platforms such as WebAssembly and Asm.JS)
        EntityContainer {
            entities: Slab::new(),
            positions: vec![Vec::new(); universe_size * universe_size],
        }
    }

    /// Inserts an entity into the container, returning its index
    pub fn insert(&mut self, entity: Entity<C, E, M>, universe_index: usize) -> usize {
        let entity_id = self.entities.insert((entity, universe_index));

        // update the positions vector to store the index of the entity
        self.positions[universe_index].push(entity_id);

        entity_id
    }

    /// Removes an entity from the container, returning it along with its current index in the
    /// universe.
    pub fn remove(&mut self, entity_index: usize) -> Entity<C, E, M> {
        let (entity, universe_index) = self.entities.remove(entity_index);

        // find the index of the index pointer inside the location vector and remove it
        let position_index = self.positions[universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect(
                "Unable to locate entity index at the expected location in the positions vector!",
            );
        let removed_entity_index = self.positions[universe_index].remove(position_index);
        debug_assert_eq!(removed_entity_index, entity_index);

        entity
    }

    /// Returns a reference to the entity contained at the supplied index.  Will cause undefined
    /// behavior in release mode and panic in debug mode ifthe index is out of bounds or the
    /// slot at the specified index is empty.
    pub unsafe fn get(&self, index: usize) -> &Entity<C, E, M> { &self.entities[index].0 }

    /// Returns a mutable reference to the entity contained at the supplied index.  Will cause
    /// undefined behavior in release mode and panic in debug mode ifthe index is out of bounds
    /// or the slot at the specified index is empty.
    pub unsafe fn get_mut(&mut self, index: usize) -> &mut Entity<C, E, M> {
        &mut self.entities[index].0
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the
    /// supplied UUID.  If so, returns a reference to the contained entity and its corresponding
    /// universe index.
    pub fn get_verify(&self, index: usize, uuid: Uuid) -> Option<(&Entity<C, E, M>, usize)> {
        let (entity, universe_index) = &self.entities[index];
        if entity.uuid == uuid {
            Some((entity, *universe_index))
        } else {
            None
        }
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the
    /// supplied UUID.  If so, returns the a mutable reference to the contained entity and its
    /// corresponding universe index.
    pub fn get_verify_mut(
        &mut self,
        index: usize,
        uuid: Uuid,
    ) -> Option<(&mut Entity<C, E, M>, usize)> {
        let (entity, universe_index) = &mut self.entities[index];
        if entity.uuid == uuid {
            Some((entity, *universe_index))
        } else {
            None
        }
    }

    /// Moves an entity from one location in the universe to another.  This function assumes that
    /// the supplied index is occupied and that the destination index is sane.
    pub fn move_entity(&mut self, entity_index: usize, dst_universe_index: usize) {
        debug_assert!(entity_index < self.entities.len());
        let src_universe_index: &mut usize = &mut self.entities[entity_index].1;

        // remove the index of the entity from the old universe index
        let position_index = self.positions[*src_universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect(
                "Unable to locate entity index at the expected location in the positions vector!",
            );
        let removed = self.positions[*src_universe_index].remove(position_index);
        debug_assert_eq!(entity_index, removed);
        // and add it to the new universe index in the position vector
        self.positions[dst_universe_index].push(entity_index);
        *src_universe_index = dst_universe_index;
    }

    /// Creates an iterator over the entities contained within the container with the format
    /// `(Entity, entity_index, universe_index)`.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&Entity<C, E, M>, usize, usize)> {
        self.entities
            .iter()
            .map(|(entity_ix, (entity, universe_ix))| (entity, entity_ix, *universe_ix))
    }

    /// Creates a mutable iterator over the entities contained within the container with the format
    /// `(Entity, entity_index, universe_index)`.
    // pub fn iter_mut<'a>(
    //     &'a mut self,
    // ) -> impl Iterator<Item = (&'a mut Entity<C, E, M>, usize, usize)> {
    //     self.entities
    //         .iter_mut()
    //         .enumerate()
    //         .filter(|&(_, ref slot)| match slot {
    //             &&mut EntitySlot::Occupied {
    //                 entity: _,
    //                 universe_index: _,
    //             } => true,
    //             &&mut EntitySlot::Empty(_) => false,
    //         })
    //         .map(|(entity_index, slot)| match slot {
    //             &mut EntitySlot::Occupied {
    //                 ref mut entity,
    //                 ref universe_index,
    //             } => (entity, entity_index, *universe_index),
    //             _ => unreachable!(),
    //         })
    // }

    /// Returns the position of the entity with the given entity index.
    pub fn get_position_index(&self, entity_index: usize) -> usize {
        debug_assert!(self.entities.len() > entity_index);
        let universe_index = self.entities[entity_index].1;

        self.positions[universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect(
                "Unable to find entry in position vector at at index pointed to by entity vector!",
            )
    }

    /// Returns a reference to the slice of all the `entity_index`es of all entities at a certain
    /// universe index.
    pub fn get_entities_at(&self, universe_index: usize) -> &[usize] {
        &self.positions[universe_index]
    }

    pub fn len(&self) -> usize { self.entities.len() }
}
