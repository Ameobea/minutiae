//! An attempt at a sparse universe that only keeps track of cells that have actually changed.  It turns out that
//! this implementation at least was incredibly, ridiculously slow and not in any way worth it.
//!
//! Don't use this.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::marker::PhantomData;

use minutiae::prelude::*;
use minutiae::universe::{CellContainer, ContiguousUniverse, Into2DIndex};
use minutiae::util::get_coords;
#[cfg(test)]
use test;

use super::UNIVERSE_SIZE;

/// A world generator that can generate the initial values for arbitrary cells on demand without
/// needing to generate surrounding cells.
pub trait CellGenerator<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    I: Ord,
> {
    fn gen_cell(universe_index: I) -> Cell<CS>;

    fn gen_initial_entities(universe_index: I) -> Vec<Entity<CS, ES, MES>>;
}

/// Small market struct that can get a `CellGenerator` to function as a normal `Generator`.
pub struct CellGeneratorWrapper<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    I: Ord + Into2DIndex,
    CG: CellGenerator<CS, ES, MES, I>,
>(
    PhantomData<CS>,
    PhantomData<ES>,
    PhantomData<MES>,
    PhantomData<I>,
    PhantomData<CG>
);

impl<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    I: Ord + Into2DIndex,
    CG: CellGenerator<CS, ES, MES, I>,
> CellGeneratorWrapper<CS, ES, MES, I, CG> {
    pub fn new() -> Self {
        CellGeneratorWrapper(
            PhantomData,
            PhantomData,
            PhantomData,
            PhantomData,
            PhantomData
        )
    }
}

impl<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    I: Ord + Into2DIndex,
    CG: CellGenerator<CS, ES, MES, I>,
> Generator<CS, ES, MES> for CellGeneratorWrapper<CS, ES, MES, I, CG> {
    fn gen(&mut self, conf: &Universe2DConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        let cells = (0..(conf.size * conf.size) as usize)
            .map(|universe_index: usize| -> I { I::from_2d_index(UNIVERSE_SIZE, universe_index) })
            .map(|coord| CG::gen_cell(coord))
            .collect();

        let entities = (0..(conf.size * conf.size) as usize)
            .map(|universe_index: usize| -> I { I::from_2d_index(UNIVERSE_SIZE, universe_index) })
            .map(|coord| CG::gen_initial_entities(coord))
            .collect();

        (cells, entities)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct P2D {
    pub x: usize,
    pub y: usize,
}

impl P2D {
    pub fn get_index(&self, universe_size: usize) -> usize {
        get_index(self.x as usize, self.y as usize, universe_size)
    }

    pub fn from_index(index: usize, universe_size: usize) -> Self {
        let (x, y) = get_coords(index, universe_size);
        P2D { x, y }
    }
}

impl Ord for P2D {
    fn cmp(&self, other: &P2D) -> Ordering {
        let y_cmp: Ordering = self.y.cmp(&other.y);

        if y_cmp == Ordering::Equal {
            self.x.cmp(&other.x)
        } else {
            y_cmp
        }
    }
}

impl PartialOrd for P2D {
    fn partial_cmp(&self, other: &P2D) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Into2DIndex for P2D {
    fn into_2d_index(self, universe_size: usize) -> usize {
        self.get_index(universe_size) as usize
    }

    fn from_2d_index(universe_size: usize, universe_index: usize) -> Self {
        let (x, y) = get_coords(universe_index, universe_size);
        P2D { x, y }
    }
}

/// Defines a sparse universe that only contains modifications made to the universe from its initial
/// state as defined by the world generator.
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "CS: for<'d> ::serde::Deserialize<'d>")]
pub struct Sparse2DUniverse<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> {
    data: BTreeMap<P2D, Cell<CS>>,
    entities: EntityContainer<CS, ES, MES, P2D>,
    __phantom_es: PhantomData<ES>,
    __phantom_mes: PhantomData<MES>,
    __phantom_g: PhantomData<G>,
}

impl<
    CS: CellState + Copy + PartialEq,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> Sparse2DUniverse<CS, ES, MES, G> {
    pub fn new() -> Self {
        Sparse2DUniverse {
            data: BTreeMap::new(),
            entities: EntityContainer::new(),
            __phantom_es: PhantomData,
            __phantom_mes: PhantomData,
            __phantom_g: PhantomData,
        }
    }
}

impl<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> Default for Sparse2DUniverse<CS, ES, MES, G> {
    fn default() -> Self {
        Sparse2DUniverse {
            data: BTreeMap::new(),
            entities: EntityContainer::new(),
            __phantom_es: PhantomData,
            __phantom_mes: PhantomData,
            __phantom_g: PhantomData,
        }
    }
}

impl<
    CS: CellState + Copy + PartialEq,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> Universe<CS, ES, MES> for Sparse2DUniverse<CS, ES, MES, G> {
    type Coord = P2D;

    fn get_cell(&self, coord: Self::Coord) -> Option<Cow<Cell<CS>>> {
        self.data.get(&coord)
            .map(|c| Cow::Borrowed(c))
            .or(Some(Cow::Owned(G::gen_cell(coord))))
    }

    unsafe fn get_cell_unchecked(&self, coord: Self::Coord) -> Cow<Cell<CS>> {
        self.get_cell(coord).unwrap()
    }

    fn set_cell(&mut self, coord: Self::Coord, new_state: CS) {
        match self.data.entry(coord) {
            Entry::Occupied(mut occupied) => {
                let default_cell = G::gen_cell(coord);

                // TODO: Investigate if doing these checks every time (as opposed to just the
                // initial time we set a value) is worth it.
                if occupied.get().state == default_cell.state {
                    occupied.remove();
                } else {
                    occupied.get_mut().state = new_state;
                }
            },
            // TODO: Investigate penalty of generating these default cells and investigate whether or
            // not this comparison is worth the memory gained.
            Entry::Vacant(empty) => if new_state != G::gen_cell(coord).state {
                empty.insert(Cell { state: new_state });
            }
        }
    }

    fn set_cell_unchecked(&mut self, coord: Self::Coord, new_state: CS) {
        self.data.insert(coord, Cell { state: new_state });
    }

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<CS, ES, MES, P2D> {
        &self.entities
    }

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<CS, ES, MES, P2D> {
        &mut self.entities
    }

    fn empty() -> Self {
        Self::default()
    }
}

impl<
    'u,
    CS: CellState + 'static,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> CellContainer<CS, P2D> for Sparse2DUniverse<CS, ES, MES, G> {
    fn get_cell_direct(&self, coord: P2D) -> Cell<CS> {
        self.data.get(&coord)
            .map(|c| c.clone())
            .unwrap_or(G::gen_cell(coord))
    }
}

impl<
    'u,
    CS: CellState + Copy + PartialEq + 'static,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES, P2D>,
> ContiguousUniverse<
    CS, ES, MES, P2D, Self
> for Sparse2DUniverse<CS, ES, MES, G> {
    fn get_cell_container<'a>(&'a self) -> &'a Self {
        self
    }
}

#[derive(Clone)]
pub struct UniverseIterator {
    pub start: P2D,
    pub end: P2D,
    next: P2D,
}

impl UniverseIterator {
    pub fn new(start: P2D, end: P2D) -> Self {
        assert!(end.y > start.y && end.x > start.x);
        UniverseIterator { start, end, next: start }
    }
}

impl Iterator for UniverseIterator {
    type Item = P2D;

    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.next;

        if self.next.x == self.end.x {
            if self.next.y > self.end.y {
                return None;
            } else {
                self.next.y += 1;
            }
        } else {
            self.next.x += 1;
        }

        Some(cur)
    }
}

impl ExactSizeIterator for UniverseIterator {
    fn len(&self) -> usize {
        let row_length = self.end.x - self.start.x;
        let row_count = self.end.y - self.start.y;

        (row_length * row_count) as usize
    }
}

#[bench]
fn sparse_universe_access(bencher: &mut test::Bencher) {
    use std::mem::size_of;

    use super::*;

    println!("Size of `CS`: {}", size_of::<CS>());

    const UNIVERSE_SIZE: usize = 10_000;

    struct DummyGen;

    impl CellGenerator<CS, ES, MES, P2D> for DummyGen {
        fn gen_cell(_: P2D) -> Cell<CS> {
            Cell { state: CS::default() }
        }

        fn gen_initial_entities(_: P2D) -> Vec<Entity<CS, ES, MES>> {
            Vec::new()
        }
    }

    let mut uni: Sparse2DUniverse<
        CS, ES, MES, DummyGen
    > = Sparse2DUniverse::new(DummyGen);

    // Initialze the universe with some values
    for i in 0..(UNIVERSE_SIZE / 2) {
        uni.set_cell_unchecked(P2D::from_index(i, UNIVERSE_SIZE), CS::__placeholder2);
    }

    bencher.iter(|| {
        let cell: Cell<CS> = uni
            .get_cell(P2D { x: 0, y: 0, })
            .unwrap()
            .into_owned();

        assert_eq!(cell, Cell { state: CS::__placeholder2 });
    })
}
