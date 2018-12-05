//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by
//! a square two-dimensional array of cells.  The universe has a set view distance which determines
//! what range cells and entities have information about their neighbors; a view distance of 0 means
//! they only have knowledge of their own state, a view distance of 1 means that they have knowledge
//! of all neighbors touching them (including diagonals), etc.

use std::borrow::Cow;

#[cfg(feature = "serde")]
use serde::Deserialize;

use cell::{Cell, CellState};
use container::EntityContainer;
use entity::{EntityState, MutEntityState};
use generator::Generator;

pub trait Universe<C: CellState, E: EntityState<C>, M: MutEntityState>: Default {
    fn get_cell(&self, coord: usize) -> Option<Cow<Cell<C>>>;

    unsafe fn get_cell_unchecked(&self, coord: usize) -> Cow<Cell<C>>;

    fn set_cell(&mut self, coord: usize, new_state: C);

    fn set_cell_unchecked(&mut self, coord: usize, new_state: C);

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<C, E, M>;

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<C, E, M>;

    fn get_cells<'a>(&'a self) -> &'a [Cell<C>];

    fn get_cells_mut<'a>(&'a mut self) -> &'a mut [Cell<C>];

    fn empty() -> Self;
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Universe2DConf {
    pub size: u32,
}

impl Default for Universe2DConf {
    fn default() -> Universe2DConf { Universe2DConf { size: 800 } }
}

// #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone)]
// #[cfg_attr(feature = "serde", serde(bound = "C: for<'d> Deserialize<'d>"))]
pub struct Universe2D<C: CellState, E: EntityState<C>, M: MutEntityState> {
    pub conf: Universe2DConf,
    pub cells: Vec<Cell<C>>,
    pub entities: EntityContainer<C, E, M>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Universe2D<C, E, M> {
    pub fn new(conf: Universe2DConf, gen: &mut Generator<C, E, M>) -> Universe2D<C, E, M> {
        assert!(conf.size > 0);
        let universe_size = conf.size as usize;

        let mut universe = Universe2D {
            conf,
            cells: Vec::new(),
            entities: EntityContainer::new(universe_size),
        };

        // use the generator to generate an initial layout of cells and entities with which to
        // populate the world
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
    pub fn uninitialized() -> Self {
        let conf = Universe2DConf::default();
        let universe_size = conf.size as usize;

        Universe2D {
            conf,
            cells: Vec::new(),
            entities: EntityContainer::new(universe_size),
        }
    }

    pub fn get_conf<'a>(&'a self) -> &'a Universe2DConf { &self.conf }

    pub fn get_size(&self) -> usize { self.conf.size as usize }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Default for Universe2D<C, E, M> {
    fn default() -> Self { Universe2D::uninitialized() }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Universe<C, E, M> for Universe2D<C, E, M> {
    fn get_cell(&self, coord: usize) -> Option<Cow<Cell<C>>> {
        self.cells.get(coord).map(|c| Cow::Borrowed(c))
    }

    unsafe fn get_cell_unchecked(&self, coord: usize) -> Cow<Cell<C>> {
        Cow::Borrowed(self.cells.get_unchecked(coord))
    }

    fn set_cell(&mut self, coord: usize, new_state: C) {
        match self.cells.get_mut(coord) {
            Some(&mut Cell { ref mut state }) => *state = new_state,
            None => (),
        }
    }

    fn set_cell_unchecked(&mut self, coord: usize, new_state: C) {
        self.cells[coord].state = new_state;
    }

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<C, E, M> { &self.entities }

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<C, E, M> { &mut self.entities }

    fn get_cells<'a>(&'a self) -> &'a [Cell<C>] { &self.cells }

    fn get_cells_mut<'a>(&'a mut self) -> &'a mut [Cell<C>] { &mut self.cells }

    fn empty() -> Self { Self::uninitialized() }
}
