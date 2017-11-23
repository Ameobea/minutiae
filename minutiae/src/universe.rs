//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::fmt::{self, Debug, Display, Formatter};
use std::marker::PhantomData;
use std::usize;

use uuid::Uuid;

use container::EntityContainer;
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use generator::Generator;
use action::{CellAction, SelfAction, EntityAction};

#[derive(Clone)]
pub struct UniverseConf {
    pub view_distance: usize,
    pub size: usize,
}

impl Default for UniverseConf {
    fn default() -> UniverseConf {
        UniverseConf {
            view_distance: 1,
            size: 8000,
        }
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
        conf: UniverseConf,
        gen: &mut Generator<C, E, M, CA, EA>,
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

        let universe_size = conf.size;
        let mut universe = Universe {
            conf: conf,
            cell_mutator: cell_mutator,
            entity_driver: entity_driver,
            seq: 0,
            cells: Vec::new(),
            entities: EntityContainer::new(universe_size),
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

    /// Creates a new shell universe without any defined logic designed for use in a hybrid client.
    pub fn uninitialized(universe_size: usize) -> Self {
        let dummy_driver = |
            _: usize,
            _: &Entity<C, E, M>,
            _: &EntityContainer<C, E, M>,
            _: &[Cell<C>],
            _: &mut FnMut(CA, usize),
            _: &mut FnMut(SelfAction<C, E, EA>),
            _: &mut FnMut(EA, usize, Uuid)
        | {};

        Universe {
            conf: UniverseConf::default(),
            cell_mutator: |_: usize, _: &[Cell<C>]| -> Option<C> { None },
            entity_driver: dummy_driver,
            seq: 0,
            cells: Vec::new(),
            entities: EntityContainer::new(universe_size),
            average_actions_per_cycle: 0,
            total_actions: 0,
            average_unique_entities_modified_per_cycle: 0,
            total_entity_modifications: 0,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
        }
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
                let entity_state = unsafe { &self.entities.get(entity_index).state };
                buf.push_str(&format!("{}", entity_state));
            } else {
                buf.push_str(&format!("{}", self.cells[i].state));
            }
        }

        buf.push('\n');
        write!(formatter, "{}", buf)
    }
}
