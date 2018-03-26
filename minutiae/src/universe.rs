//! Declares the universe in which all parts of the system reside.  ItThe universe is represented by a square two-dimensional
//! array of cells.  The universe has a set view distance which determines what range cells and entities have information
//! about their neighbors; a view distance of 0 means they only have knowledge of their own state, a view distance of
//! 1 means that they have knowledge of all neighbors touching them (including diagonals), etc.

use std::borrow::Cow;

use container::EntityContainer;
use cell::{Cell, CellState};
use entity::{EntityState, MutEntityState};
use generator::Generator;

pub trait Universe<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> : Default {
    /// The data type that can be used to index the cells of the universe.  For a 2D universe, it would be `usize` or `isize`.
    /// For a 3D universe, it would be `(usize, usize, usize)` or `(isize, isize, isize)` or perhaps `Point3D`.
    type Coord : Ord + Copy;

    fn get_cell(&self, coord: Self::Coord) -> Option<Cow<Cell<C>>>;

    unsafe fn get_cell_unchecked(&self, coord: Self::Coord) -> Cow<Cell<C>>;

    fn set_cell(&mut self, coord: Self::Coord, new_state: C);

    fn set_cell_unchecked(&mut self, coord: Self::Coord, new_state: C);

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<C, E, M, Self::Coord>;

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<C, E, M, Self::Coord>;

    // fn get_entities_at(&self, coord: Self::Coord) -> &[usize] {
    //     self.get_entities().get_entities_at(coord.into())
    // }

    fn empty() -> Self;
}

pub trait CellContainer<
    C: CellState + 'static,
    I: 'static,
> {
    fn get_cell_direct(&self, index: I) -> Cell<C>;
}

/// Represents universes that store their cells as a flat buffer that can be accessed as a vector.
pub trait ContiguousUniverse<
    C: CellState + 'static,
    E: EntityState<C>,
    M: MutEntityState,
    I: Ord + 'static,
    CC: CellContainer<C, I>,
> {
    fn get_cell_container<'a>(&'a self) -> &'a CC;
}

#[derive(Clone)]
pub struct Universe2DConf {
    pub view_distance: usize,
    pub size: usize,
}

impl Default for Universe2DConf {
    fn default() -> Universe2DConf {
        Universe2DConf {
            view_distance: 1,
            size: 8000,
        }
    }
}

pub struct Universe2D<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> {
    pub conf: Universe2DConf,
    pub cells: Vec<Cell<C>>,
    pub entities: EntityContainer<C, E, M, usize>,
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> Universe2D<C, E, M> {
    pub fn new(
        conf: Universe2DConf,
        gen: &mut Generator<C, E, M>,
    ) -> Universe2D<C, E, M> {
        assert!(conf.size > 0);

        let universe_size = conf.size;
        let mut universe = Universe2D {
            conf,
            cells: Vec::new(),
            entities: EntityContainer::new(),
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
    pub fn uninitialized() -> Self {
        Universe2D {
            conf: Universe2DConf::default(),
            cells: Vec::new(),
            entities: EntityContainer::new(),
        }
    }

    pub fn get_conf<'a>(&'a self) -> &'a Universe2DConf {
        &self.conf
    }

    pub fn get_size(&self) -> usize {
        self.conf.size
    }
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> Default for Universe2D<C, E, M> {
    fn default() -> Self {
        Universe2D::uninitialized()
    }
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
> Universe<C, E, M> for Universe2D<C, E, M> {
    type Coord = usize;

    fn get_cell(&self, coord: Self::Coord) -> Option<Cow<Cell<C>>> {
        self.cells.get(coord)
            .map(|c| Cow::Borrowed(c))
    }

    unsafe fn get_cell_unchecked(&self, coord: Self::Coord) -> Cow<Cell<C>> {
        Cow::Borrowed(self.cells.get_unchecked(coord))
    }

    fn set_cell(&mut self, coord: Self::Coord, new_state: C) {
        match self.cells.get_mut(coord) {
            Some(&mut Cell { ref mut state }) => *state = new_state,
            None => (),
        }
    }

    fn set_cell_unchecked(&mut self, coord: Self::Coord, new_state: C) {
        self.cells[coord].state = new_state;
    }

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<C, E, M, usize> {
        &self.entities
    }

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<C, E, M, usize> {
        &mut self.entities
    }

    fn empty() -> Self {
        Self::uninitialized()
    }
}

impl<C: CellState + 'static> CellContainer<C, usize> for Vec<Cell<C>> {
    fn get_cell_direct(&self, index: usize) -> Cell<C> {
        self[index].clone()
    }
}

impl<
    C: CellState + 'static,
    E: EntityState<C>,
    M: MutEntityState,
> ContiguousUniverse<C, E, M, usize, Vec<Cell<C>>> for Universe2D<C, E, M> {
    fn get_cell_container<'a>(&'a self) -> &'a Vec<Cell<C>> {
        &self.cells
    }
}

pub trait Into2DIndex {
    fn into_2d_index(self, universe_size: usize) -> usize;

    fn from_2d_index(universe_size: usize, universe_index: usize) -> Self;
}
