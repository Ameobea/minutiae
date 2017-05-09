//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::cell::Cell as RustCell;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;

use uuid::Uuid;

use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use generator::Generator;
use action::{Action, CellAction, SelfAction, EntityAction};

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
    pub entities: Vec<Vec<Entity<C, E, M>>>,
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
            entities: Vec::new(),
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
        universe.entities = entities;
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

            if self.entities[i].len() > 0 {
                buf.push_str(&format!("{}", self.entities[i][0].state));
            } else {
                buf.push_str(&format!("{}", self.cells[i].state));
            }
        }

        buf.push('\n');
        write!(formatter, "{}", buf)
    }
}
