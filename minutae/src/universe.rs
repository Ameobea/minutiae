//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::cell::Cell as RustCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::Filter;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};
use std::slice::Iter;
use std::usize;


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
}

impl Index<usize> for EntityPositions {
    type Output = Vec<usize>;

    fn index<'a>(&'a self, index: usize) -> &'a Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for EntityPositions {
    fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut Self::Output {
        &mut self.0[index]
    }
}

pub enum EntitySlot<C: CellState, E: EntityState<C>, M: MutEntityState> {
    Occupied{
        entity: Entity<C, E, M>,
        universe_index: usize
    },
    Empty(usize),
}

pub struct EntityContainer<C: CellState, E: EntityState<C>, M: MutEntityState>(Vec<EntitySlot<C, E, M>>, usize, EntityPositions);
impl<C: CellState, E: EntityState<C>, M: MutEntityState> EntityContainer<C, E, M> {
    pub fn new() -> Self {
        // a point of `usize::MAX` indicates that the slot is the last available one.
        EntityContainer(vec![EntitySlot::Empty(usize::MAX)], 0, EntityPositions::new())
    }

    /// Inserts an entity into the container, returning its index
    pub fn insert(&mut self, entity: Entity<C, E, M>, universe_index: usize) -> usize {
        let &mut EntityContainer(entities, index, positions) = self;
        if index != usize::MAX {
            let next_empty = match entities[index] {
                EntitySlot::Empty(next_empty) => next_empty,
                _ => unreachable!(),
            };

            // update the index of the next empty cell and insert the entity into the empty slot
            self.1 = next_empty;
            entities[index] = EntitySlot::Occupied{entity, universe_index};
            // update the positions vector to store the index of the entity
            positions[universe_index].push(index);

            index
        } else {
            // this is the last empty slot in the container, so we have to allocate space for another
            self.0.push(EntitySlot::Occupied{entity, universe_index});

            self.0.len() - 1
        }
    }

    /// Removes an entity from the container, returning it along with its current index in the universe.
    pub fn remove(&mut self, index: usize) -> Entity<C, E, M> {
        let removed = mem::replace(&mut self.0[index], EntitySlot::Empty(self.1));
        self.1 = index;

        match removed {
            EntitySlot::Occupied{entity, universe_index} => {
                // find the index of the index pointer inside the location vector and remove it
                let entity_index = self.2[universe_index]
                    .iter()
                    .position(|&index| index == universe_index)
                    .expect("Unable to locate entity index at the expected location in the positions vector!");
                self.2[universe_index].remove(entity_index);

                entity
            },
            EntitySlot::Empty(_) => unreachable!(),
        }
    }

    /// Returns a reference to the entity contained at the supplied index
    pub fn get(&mut self, index: usize) -> &Entity<C, E, M> {
        match self.0[index] {
            EntitySlot::Occupied{entity, universe_index} => &entity,
            _ => unreachable!(),
        }
    }

    /// Moves an entity from one location in the universe to another
    pub fn move_entity(&mut self, entity_index: usize, dst_universe_index: usize) {
        let src_universe_index = *match self.0[entity_index] {
            EntitySlot::Occupied{entity, ref mut universe_index} => {
                // update the universe index within the entity slot
                let src_index = universe_index;
                *universe_index = dst_universe_index;
                src_index
            },
            _ => unreachable!(),
        };

        // remove the index of the entity from the old universe index
        let position_index = self.2[src_universe_index]
            .iter()
            .position(|&index| index == entity_index)
            .expect("Unable to locate entity index at the expected location in the positions vector!");
        self.2[src_universe_index].remove(position_index);
        // and add it to the new universe index in the position vector
        self.2[dst_universe_index].push(entity_index);
    }

    pub fn iter(&self) -> Filter<EntitySlot<C, E, M>> {
        self.0.iter()
            .filter(|slot| match slot {
                &&EntitySlot::Occupied{entity: _, universe_index: _} => true,
                &&EntitySlot::Empty(_) => false,
            })
    }
}

pub struct Universe<C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>> {
    pub conf: UniverseConf,
    // function for transforming a cell to the next state given itself and an array of its neigbors
    pub cell_mutator: fn(usize, &[Cell<C>]) -> Option<C>,
    // function that determines the behaviour of entities.
    pub entity_driver: fn(
        cur_x: usize,
        cur_x: usize,
        entity: &E,
        mut_state: &RustCell<M>,
        entities: &[Vec<Entity<C, E, M>>],
        cells: &[Cell<C>],
        cell_action_executor: &mut FnMut(CA, isize, isize),
        self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
        entity_action_executor: &mut FnMut(EA, isize, isize, Uuid)
    ),

    pub seq: usize,
    pub cells: Vec<Cell<C>>,
    pub entities: EntityContainer<C, E, M>,
    // Contains the indices of all grid cells that contain entities.
    pub entity_meta: HashMap<Uuid, (usize, usize)>,
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
            cur_x: usize,
            cur_x: usize,
            entity: &E,
            mut_state: &RustCell<M>,
            entities: &[Vec<Entity<C, E, M>>],
            cells: &[Cell<C>],
            cell_action_executor: &mut FnMut(CA, isize, isize),
            self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
            entity_action_executor: &mut FnMut(EA, isize, isize, Uuid)
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
            entity_meta: HashMap::new(),
            average_actions_per_cycle: 0,
            total_actions: 0,
            average_unique_entities_modified_per_cycle: 0,
            total_entity_modifications: 0,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
        };

        // use the generator to generate an initial layout of cells and entities with which to populate the world
        let (cells, entities, entity_meta) = gen.gen(&universe.conf);

        universe.cells = cells;
        // universe.entities = entities; // TODO
        universe.entity_meta = entity_meta;

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

            // if self.entities[i].len() > 0 { // TODO
            //     buf.push_str(&format!("{}", self.entities[i][0].state));
            // } else {
            //     buf.push_str(&format!("{}", self.cells[i].state));
            // }
        }

        buf.push('\n');
        write!(formatter, "{}", buf)
    }
}
