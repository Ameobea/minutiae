//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::marker::PhantomData;

use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use engine::Engine;
use generator::Generator;
use action::{CellAction, EntityAction};

#[derive(Clone)]
pub struct UniverseConf {
    pub view_distance: usize,
    pub size: usize,
    pub overlapping_entities: bool, // if true, multiple entities can reside on the same coordinate simulaneously.
}

impl Default for UniverseConf {
    fn default() -> UniverseConf {
        UniverseConf {
            view_distance: 1,
            size: 10000,
            overlapping_entities: true,
        }
    }
}

pub struct Universe<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, CA, EA>> {
    pub conf: UniverseConf,
    // function for transforming a cell to the next state given itself and an array of its neigbors
    pub cell_mutator: Box<for <'a> Fn(&'a Cell<C>, &Fn(isize, isize) -> Option<&'a Cell<C>>) -> Cell<C>>,
    // generator: Box<Generator<C, E, N>>,
    pub engine: Box<N>,

    pub seq: usize,
    pub cells: Vec<Cell<C>>,
    pub entities: Vec<Vec<Entity<C, E>>>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
}

impl<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, CA, EA>> Universe<C, E, CA, EA, N> {
    pub fn new(
        conf: UniverseConf, gen: &mut Generator<C, E, CA, EA, N>, engine: Box<N>,
        cell_mutator: Box<for <'a> Fn(&'a Cell<C>, &Fn(isize, isize) -> Option<&'a Cell<C>>) -> Cell<C>>,
    ) -> Universe<C, E, CA, EA, N> {
        assert!(conf.size > 0);

        let mut universe = Universe {
            conf: conf,
            cell_mutator: cell_mutator,
            engine: engine,
            seq: 0,
            cells: Vec::new(),
            entities: Vec::new(),
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
        };

        // use the generator to generate an initial layout of cells and entities with which to populate the world
        let (cells, entities) = gen.gen(&universe.conf);

        universe.cells = cells;
        universe.entities = entities;

        universe
    }
}
