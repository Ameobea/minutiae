use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::marker::PhantomData;

use minutiae::util::get_index as get_index_util;

use super::*;

/// A world generator that can generate the initial values for arbitrary cells on demand without
/// needing to generate surrounding cells.
pub trait CellGenerator<
    CS: CellState,
    ES: EntityState<CS>,
    MES: MutEntityState
> {
    fn gen_cell(&self, universe_index: usize) -> Cell<CS>;

    fn gen_cell_ref<'a>(&'a self, universe_index: usize) -> &'a Cell<CS>;

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

}

fn get_index(coord: (usize, usize)) -> usize {
    get_index_util(coord.0, coord.1, UNIVERSE_SIZE)
}

impl<
    'b,
    CS: CellState + PartialEq,
    ES: EntityState<CS>,
    MES: MutEntityState,
    G: CellGenerator<CS, ES, MES>,
> Universe<CS, ES, MES> for Sparse2DUniverse<CS, ES, MES, G> {
    type Coord = (usize, usize);

    fn get_cell<'a>(&'a self, coord: Self::Coord) -> Option<&'a Cell<CS>> {
        let index = get_index(coord);
        self.data
            .get(&index)
            .or(Some(self.gen.gen_cell_ref(index)))
    }

    unsafe fn get_cell_unchecked<'a>(&'a self, coord: Self::Coord) -> &'a Cell<CS> {
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
