//! Small ant colony simulation with pheremone trails and simulated foraging behavior.

extern crate minutiae;
extern crate uuid;

use minutiae::prelude::*;
use uuid::Uuid;

extern {
    pub fn canvas_render(pixbuf_ptr: *const u8);
}

#[derive(Clone)]
struct Pheremones {
    searching: u16,
    found: u16,
}

#[derive(Clone)]
enum CellContents {
    Empty,
    Filled(u8),
    Food,
}

#[derive(Clone)]
struct CS {
    pheremones: Pheremones,
    contents: CellContents,
}

impl CellState for CS {}

#[derive(Clone)]
struct ES {}

impl EntityState<CS> for ES {}

#[derive(Clone, Default)]
struct MES {}

impl MutEntityState for MES {}

fn color_calculator(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    unimplemented!(); // TODO
}

struct CA;

impl CellAction<CS> for CA {}

struct EA;

impl EntityAction<CS, ES> for EA {}

struct WorldGenerator;

impl Generator<CS, ES, MES, CA, EA> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        unimplemented!(); // TODO
    }
}

fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

fn entity_driver(
    universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {
    unimplemented!(); // TODO
}

fn main() {
    let conf = UniverseConf {
        iter_cells: false,
        size: 800,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WorldGenerator, cell_mutator, entity_driver);
}
