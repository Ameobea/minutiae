//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};
use std::usize;

use uuid::Uuid;

use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use generator::Generator;
use action::{CellAction, SelfAction, EntityAction};

#[derive(Clone)]
pub struct UniverseConf {
    pub view_distance: usize,
    pub size: usize,
    pub iter_cells: bool,
}

impl Default for UniverseConf {
    fn default() -> UniverseConf {
        UniverseConf {
            view_distance: 1,
            size: 8000,
            iter_cells: false,
        }
    }
}

// TODO: Move entity containers into their own file

/// For each coordinate on the grid, keeps track of the entities that inhabit it by holding a list of
/// indexes to slots in the `EntityContainer`.
pub struct EntityPositions(Vec<Vec<usize>>);

impl EntityPositions {
    pub fn new() -> Self {
        EntityPositions(Vec::new())
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

pub enum EntitySlot<C: CellState, E: EntityState<C>, M: MutEntityState> {
    Occupied{
        entity: Entity<C, E, M>,
        universe_index: usize
    },
    Empty(usize),
}

pub struct EntityContainer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    entities: Vec<EntitySlot<C, E, M>>,
    empty_index: usize,
    positions: EntityPositions
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> EntityContainer<C, E, M> {
    pub fn new() -> Self {
        // a point of `usize::MAX` indicates that the slot is the last available one.
        EntityContainer{
            entities: vec![EntitySlot::Empty(usize::MAX)],
            empty_index: 0,
            positions: EntityPositions::new()
        }
    }

    /// Inserts an entity into the container, returning its index
    pub fn insert(&mut self, entity: Entity<C, E, M>, universe_index: usize) -> usize {
        let &mut EntityContainer{ref mut entities, empty_index, ref mut positions} = self;
        if empty_index != usize::MAX {
            let next_empty = match entities[empty_index] {
                EntitySlot::Empty(next_empty) => next_empty,
                _ => unreachable!(),
            };

            // update the index of the next empty cell and insert the entity into the empty slot
            self.empty_index = next_empty;
            entities[empty_index] = EntitySlot::Occupied{entity, universe_index};
            // update the positions vector to store the index of the entity
            positions[universe_index].push(empty_index);

            empty_index
        } else {
            // this is the last empty slot in the container, so we have to allocate space for another
            let entity_count = entities.len();
            entities.push(EntitySlot::Occupied{entity, universe_index});

            entity_count
        }
    }

    /// Removes an entity from the container, returning it along with its current index in the universe.
    pub fn remove(&mut self, index: usize) -> Entity<C, E, M> {
        let removed = mem::replace(&mut self.entities[index], EntitySlot::Empty(self.empty_index));
        self.empty_index = index;

        match removed {
            EntitySlot::Occupied{entity, universe_index} => {
                // find the index of the index pointer inside the location vector and remove it
                let entity_index = self.positions[universe_index]
                    .iter()
                    .position(|&index| index == universe_index)
                    .expect("Unable to locate entity index at the expected location in the positions vector!");
                self.positions[universe_index].remove(entity_index);

                entity
            },
            EntitySlot::Empty(_) => unreachable!(),
        }
    }

    /// Returns a reference to the entity contained at the supplied index
    pub fn get(&self, index: usize) -> &Entity<C, E, M> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked(index) } {
            &EntitySlot::Occupied{ref entity, universe_index: _} => entity,
            _ => unreachable!(),
        }
    }

    /// Checks if 1) an entity exists at the provided index and 2) that its UUID matches the supplied UUID.  If so, returns
    /// the contained entity and its corresponding universe index.
    pub fn get_verify(&self, index: usize, uuid: Uuid) -> Option<(&Entity<C, E, M>, usize)> {
        debug_assert!(index < self.entities.len());
        match unsafe { self.entities.get_unchecked(index) } {
            &EntitySlot::Occupied{ref entity, universe_index} => {
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
                let src_index: usize = *universe_index;
                *universe_index = dst_universe_index;

                src_index
            },
            _ => unreachable!(),
        };

        // remove the index of the entity from the old universe index
        let position_index = self.positions[src_universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect("Unable to locate entity index at the expected location in the positions vector!");
        self.positions[src_universe_index].remove(position_index);
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
}

pub struct Universe<C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>> {
    pub conf: UniverseConf,
    // function for transforming a cell to the next state given itself and an array of its neigbors
    pub cell_mutator: fn(usize, &[Cell<C>]) -> Option<C>,
    // function that determines the behaviour of entities.
    pub entity_driver: fn(
        universe_index: usize,
        entity: &Entity<C, E, M>,
        entities: &EntityContainer<C, E, M>,
        cells: &[Cell<C>],
        cell_action_executor: &mut FnMut(CA, usize),
        self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
        entity_action_executor: &mut FnMut(EA, usize, Uuid)
    ),

    pub seq: usize,
    pub cells: Vec<Cell<C>>,
    pub entities: EntityContainer<C, E, M>,
    // these two values are used for pre-allocating space on the action buffer based on average actions per cycle
    pub average_actions_per_cycle: usize,
    pub total_actions: usize,
    pub average_unique_entities_modified_per_cycle: usize,
    pub total_entity_modifications: usize,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>> Universe<C, E, M, CA, EA> {
    pub fn new(
        conf: UniverseConf, gen: &mut Generator<C, E, M, CA, EA>,
        cell_mutator: fn(usize, &[Cell<C>]) -> Option<C>,
        entity_driver: fn(
            universe_index: usize,
            entity: &Entity<C, E, M>,
            entities: &EntityContainer<C, E, M>,
            cells: &[Cell<C>],
            cell_action_executor: &mut FnMut(CA, usize),
            self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
            entity_action_executor: &mut FnMut(EA, usize, Uuid)
        )
    ) -> Universe<C, E, M, CA, EA> {
        assert!(conf.size > 0);

        let mut universe = Universe {
            conf: conf,
            cell_mutator: cell_mutator,
            entity_driver: entity_driver,
            seq: 0,
            cells: Vec::new(),
            entities: EntityContainer::new(),
            average_actions_per_cycle: 0,
            total_actions: 0,
            average_unique_entities_modified_per_cycle: 0,
            total_entity_modifications: 0,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
        };

        // use the generator to generate an initial layout of cells and entities with which to populate the world
        let (cells, entities) = gen.gen(&universe.conf);

        universe.cells = cells;
        for (universe_index, entity_vec) in entities.into_iter().enumerate() {
            for entity in entity_vec {
                universe.entities.insert(entity, universe_index);
            }
        }

        universe
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>
> Debug for Universe<C, E, M, CA, EA> where C:Display, E:Display {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let length = self.conf.size * self.conf.size;
        let mut buf = String::with_capacity(length);
        buf.push_str(&format!("{}", self.seq));

        for i in 0..length {
            if i % self.conf.size == 0 {
                buf.push('\n');
            }

            let entities = self.entities.get_entities_at(i);
            if entities.len() > 0 { // TODO
                let entity_index = entities[0];
                let entity_state = &self.entities.get(entity_index).state;
                buf.push_str(&format!("{}", entity_state));
            } else {
                buf.push_str(&format!("{}", self.cells[i].state));
            }
        }

        buf.push('\n');
        write!(formatter, "{}", buf)
    }
}
