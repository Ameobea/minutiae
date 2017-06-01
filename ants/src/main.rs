//! Small ant colony simulation with pheremone trails and simulated foraging behavior.

extern crate minutiae;
extern crate uuid;

use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialGridIterator, SerialEntityIterator};
use minutiae::driver::BasicDriver;
use minutiae::driver::middleware::MinDelay;
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use uuid::Uuid;

extern {
    pub fn canvas_render(pixbuf_ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 800;

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

/// No-op cell mutator since we aren't mutating cells in this simulation
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

struct AntEngine;

fn exec_cell_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

fn exec_entity_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

impl SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>> for AntEngine {
    fn iter_cells(&self, cells: &[Cell<CS>]) -> SerialGridIterator {
        SerialGridIterator::new(UNIVERSE_SIZE)
    }

    fn iter_entities(&self, entities: &[Vec<Entity<CS, ES, MES>>]) -> SerialEntityIterator<CS, ES> {
        SerialEntityIterator::new(UNIVERSE_SIZE)
    }

    fn exec_actions(
        &self, universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
        self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
    ) {
        for cell_action in cell_actions { exec_cell_action(cell_action); }
        for self_action in self_actions { exec_self_action(self_action); }
        for entity_action in entity_actions { exec_entity_action(entity_action); }
    }
}

/// Given a coordinate of the universe, uses state of its cell and the entities that reside in it to determine a color
/// to display on the canvas.  This is called each tick.  The returned value is the color in RGBA.
fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    unimplemented!(); // TODO
}

fn main() {
    let conf = UniverseConf {
        iter_cells: false,
        size: 800,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WorldGenerator, cell_mutator, entity_driver);
    let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(AntEngine);
    let driver = EmscriptenDriver;
    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
