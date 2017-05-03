//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use engine::Engine;
use generator::Generator;
use util::get_coords;

pub struct Universe<C: CellState, E: EntityState<C>, N: Engine<C, E>> {
    pub conf: UniverseConf,
    // function for transforming a cell to the next state given itself and an array of its neigbors
    pub cell_mutator: Box<Fn(&Cell<C>, &[&Cell<C>]) -> Cell<C>>,
    // generator: Box<Generator<C, E, N>>,
    pub engine: Box<N>,

    pub seq: usize,
    pub cells: Vec<Cell<C>>,
    pub entities: Vec<Vec<Entity<C, E>>>,
}

#[derive(Clone)]
pub struct UniverseConf {
    pub view_distance: usize,
    pub size: usize,
    pub overlapping_entities: bool, // if true, multiple entities can reside on the same coordinate simulaneously.
}

impl<C: CellState, E: EntityState<C>, N: Engine<C, E>> Universe<C, E, N> {
    pub fn new(
        gen: &mut Generator<C, E, N>, engine: Box<N>, conf: UniverseConf, cell_mutator: Box<Fn(&Cell<C>, &[&Cell<C>]) -> Cell<C>>
    ) -> Universe<C, E, N> {
        assert!(conf.size > 0);

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

    pub fn get_cell_neighbors(&self, index: usize) -> &[&Cell<C>] {
        unimplemented!();
    }

    pub fn get_entity_neighbors<'a>(&'a self, index: usize, buf: &mut[usize]) {
        let UniverseConf{view_distance, size, overlapping_entities: _} = self.conf;
        assert!(buf.len() == view_distance * view_distance);

        let (x, y) = get_coords(index, size);
        let min_x = if view_distance < x { x - view_distance } else { 0 };
        let max_x = if x + view_distance < size { x + view_distance } else { size - 1 };
        let min_y = if view_distance < y { y - view_distance } else { 0 };
        let max_y = if y + view_distance < size { y + view_distance } else { size - 1 };

        let mut i = 0;
        for y in min_y..max_y {
            for x in min_x..max_x {
                buf[i] = (y * size) + x;
                i += 1;
            }
        }
    }
}
