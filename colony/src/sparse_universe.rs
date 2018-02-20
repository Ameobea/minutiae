use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::marker::PhantomData;

use minutiae::util::{get_coords, get_index as get_index_util};

use super::*;

/// A world generator that can generate the initial values for arbitrary cells on demand without
/// needing to generate surrounding cells.
pub trait CellGenerator<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState
> {
    fn gen_cell(&self, universe_index: usize) -> Cell<CS>;

    fn gen_initial_entities(&self, universe_index: usize) -> Vec<Entity<CS, ES, MES>>;
}

impl<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState
> Generator<CS, ES, MES> for CellGenerator<CS, ES, MES> {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        unimplemented!()
    }
}

/// Defines a sparse universe that only contains modifications made to the universe from its initial
/// state as defined by the world generator.
pub struct Sparse2DUniverse<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES>,
> {
    data: BTreeMap<usize, Cell<CS>>,
    gen: G,
    entities: EntityContainer<CS, ES, MES>,
    __phantom_es: PhantomData<ES>,
    __phantom_mes: PhantomData<MES>,
}

impl<
    CS: CellState + PartialEq,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES>,
> Sparse2DUniverse<CS, ES, MES, G> {
    pub fn new(gen: G, universe_size: usize) -> Self {
        Sparse2DUniverse {
            data: BTreeMap::new(),
            gen,
            entities: EntityContainer::new(universe_size),
            __phantom_es: PhantomData,
            __phantom_mes: PhantomData,
        }
    }
}

fn get_index(coord: (usize, usize)) -> usize {
    get_index_util(coord.0, coord.1, UNIVERSE_SIZE)
}

impl<
    'b,
    CS: CellState + Copy + PartialEq,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES>,
> Universe<CS, ES, MES> for Sparse2DUniverse<CS, ES, MES, G> {
    type Coord = (usize, usize);

    fn get_cell(&self, coord: Self::Coord) -> Option<Cow<Cell<CS>>> {
        let index = get_index(coord);

        self.data.get(&index)
            .map(|c| Cow::Borrowed(c))
            .or(Some(Cow::Owned(self.gen.gen_cell(index))))
    }

    unsafe fn get_cell_unchecked(&self, coord: Self::Coord) -> Cow<Cell<CS>> {
        self.get_cell(coord).unwrap()
    }

    fn set_cell(&mut self, coord: Self::Coord, new_state: CS) {
        let index = get_index(coord);
        match self.data.entry(index) {
            Entry::Occupied(mut occupied) => {
                let default_cell = self.gen.gen_cell(index);

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
            Entry::Vacant(empty) => if new_state != self.gen.gen_cell(index).state {
                empty.insert(Cell { state: new_state });
            }
        }
    }

    fn set_cell_unchecked(&mut self, coord: Self::Coord, new_state: CS) {
        let index = get_index(coord);
        self.data.insert(index, Cell { state: new_state });
    }

    fn get_entities<'a>(&'a self) -> &'a EntityContainer<CS, ES, MES> {
        &self.entities
    }

    fn get_entities_mut<'a>(&'a mut self) -> &'a mut EntityContainer<CS, ES, MES> {
        &mut self.entities
    }
}

#[bench]
fn sparse_universe_access(bencher: &mut test::Bencher) {
    use std::mem::size_of;

    println!("Size of `CS`: {}", size_of::<CS>());

    const UNIVERSE_SIZE: usize = 10_000;

    struct DummyGen;

    impl CellGenerator<CS, ES, MES> for DummyGen {
        fn gen_cell(&self, _: usize) -> Cell<CS> {
            Cell { state: CS::default() }
        }

        fn gen_initial_entities(&self, _: usize) -> Vec<Entity<CS, ES, MES>> {
            unimplemented!()
        }
    }

    let mut uni: Sparse2DUniverse<
        CS, ES, MES, DummyGen
    > = Sparse2DUniverse::new(DummyGen, UNIVERSE_SIZE);

    // Initialze the universe with some values
    for i in 0..(UNIVERSE_SIZE / 2) {
        uni.set_cell_unchecked(get_coords(i, UNIVERSE_SIZE), CS::__placeholder2);
    }

    let mut i = 0;

    bencher.iter(|| {
        let cell: Cell<CS> = uni
            .get_cell(get_coords(i, UNIVERSE_SIZE))
            .unwrap()
            .into_owned();

        assert_eq!(cell, Cell { state: CS::__placeholder2 });

        if i == (UNIVERSE_SIZE / 2) {
            i = 0;
        }
    })
}
