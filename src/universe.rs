//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use engine::Engine;
use generator::Generator;

pub struct Universe<C: CellState, E: EntityState<C>, N: Engine<C, E>> {
    conf: UniverseConf,
    // function for transforming a cell to the next state given itself and an array of its neigbors
    cell_mutator: Box<Fn(Cell<C>, &[Cell<C>]) -> Cell<C>>,
    // generator: Box<Generator<C, E, N>>,
    engine: Box<N>,

    seq: usize,
    cells: Vec<Cell<C>>,
    entities: Vec<Vec<Entity<C, E>>>,
}

#[derive(Clone)]
pub struct UniverseConf {
    view_distance: usize,
    size: usize,
    overlapping_entities: bool, // if true, multiple entities can reside on the same coordinate simulaneously.
}

impl<C: CellState, E: EntityState<C>, N: Engine<C, E>> Universe<C, E, N> {
    pub fn new(gen: &mut Generator<C, E, N>, engine: Box<N>, conf: UniverseConf, cell_mutator: Box<Fn(Cell<C>, &[Cell<C>]) -> Cell<C>>) -> Universe<C, E, N> {
        let mut universe = Universe {
            conf: conf,
            cell_mutator: cell_mutator,
            engine: engine,
            seq: 0,
            cells: Vec::new(),
            entities: Vec::new(),
        };

        // use the generator to generate an initial layout of cells and entities with which to populate the world
        let (cells, entities) = gen.gen(&universe);

        universe.cells = cells;
        universe.entities = entities;

        universe
    }
}
