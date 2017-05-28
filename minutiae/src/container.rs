//! Declares container types that are used to provide abstracted access to data strucures within a universe.

use std::mem;
use std::ops::{Index, IndexMut};
use std::usize;

use uuid::Uuid;

use cell::CellState;
use entity::{Entity, EntityState, MutEntityState};

/// For each coordinate on the grid, keeps track of the entities that inhabit it by holding a list of
/// indexes to slots in the `EntityContainer`.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityPositions(pub Vec<Vec<usize>>);

#[cfg(not(feature = "serde"))]
#[derive(Clone, Debug)]
pub struct EntityPositions(pub Vec<Vec<usize>>);

impl EntityPositions {
    pub fn new(universe_size: usize) -> Self {
        EntityPositions(vec![Vec::new(); universe_size * universe_size])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Index<usize> for EntityPositions {
    type Output = Vec<usize>;

    fn index<'a>(&'a self, index: usize) -> &'a Self::Output {
        debug_assert!(index < self.0.len());
        unsafe { &self.0.get_unchecked(index) }
    }
}

impl IndexMut<usize> for EntityPositions {
    fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut Self::Output {
        debug_assert!(index < self.0.len());
        unsafe { self.0.get_unchecked_mut(index) }
    }
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EntitySlot<C: CellState, E: EntityState<C>, M: MutEntityState> {
    Occupied{
        entity: Entity<C, E, M>,
        universe_index: usize
    },
    Empty(usize),
}

#[cfg(not(feature = "serde"))]
#[derive(Clone, Debug)]
pub enum EntitySlot<C: CellState, E: EntityState<C>, M: MutEntityState> {
    Occupied{
        entity: Entity<C, E, M>,
        universe_index: usize
    },
    Empty(usize),
}

unsafe impl<C: CellState, E: EntityState<C>, M: MutEntityState> Send for EntitySlot<C, E, M> where E:Send, M:Send {}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityContainer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    pub entities: Vec<EntitySlot<C, E, M>>,
    pub empty_index: usize,
    pub positions: EntityPositions
}

#[cfg(not(feature = "serde"))]
#[derive(Clone, Debug)]
pub struct EntityContainer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    pub entities: Vec<EntitySlot<C, E, M>>,
    pub empty_index: usize,
    pub positions: EntityPositions
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> EntityContainer<C, E, M> {
    pub fn new(universe_size: usize) -> Self {
        // a point of `usize::MAX` indicates that the slot is the last available one.
        EntityContainer{
            entities: vec![EntitySlot::Empty(usize::MAX)],
            empty_index: 0,
            positions: EntityPositions::new(universe_size)
        }
    }

    /// Inserts an entity into the container, returning its index
    pub fn insert(&mut self, entity: Entity<C, E, M>, universe_index: usize) -> usize {
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
    pub fn get_verify(&self, index: usize, uuid: Uuid) -> Option<(&Entity<C, E, M>, usize)> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked(index) } {
            &EntitySlot::Occupied{ref entity, universe_index} => {
                if entity.uuid == uuid { Some((entity, universe_index)) } else { None }
            },
            _ => None,
        }
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the supplied UUID.  If so, returns
    /// the a mutable reference to the contained entity and its corresponding universe index.
    pub fn get_verify_mut(&mut self, index: usize, uuid: Uuid) -> Option<(&mut Entity<C, E, M>, usize)> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked_mut(index) } {
            &mut EntitySlot::Occupied{ref mut entity, universe_index} => {
                if entity.uuid == uuid { Some((entity, universe_index)) } else { None }
            },
            _ => None,
        }
    }

    /// Moves an entity from one location in the universe to another.  This function assumes that the supplied index
    /// is occupied and that the destination index is sane.
    pub fn move_entity(&mut self, entity_index: usize, dst_universe_index: usize) {
        debug_assert!(entity_index < self.entities.len());
        debug_assert!(dst_universe_index < self.positions.len());
        let src_universe_index: usize = match self.entities[entity_index] {
            EntitySlot::Occupied{entity: _, ref mut universe_index} => {
                // update the universe index within the entity slot
                let src_universe_index: usize = *universe_index;
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

    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(&'a Entity<C, E, M>, usize, usize)> {
        self.entities.iter()
            .enumerate()
            .filter(|&(_, slot)| match slot {
                &EntitySlot::Occupied{entity: _, universe_index: _} => true,
                &EntitySlot::Empty(_) => false,
            }).map(|(entity_index, slot)| match slot {
                &EntitySlot::Occupied{ref entity, universe_index} => (entity, entity_index, universe_index),
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
    pub fn get_entities_at(&self, universe_index: usize) -> &[usize] {
        debug_assert!(universe_index < self.positions.len());
        &self.positions[universe_index]
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }
}
