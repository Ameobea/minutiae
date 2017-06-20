//! Dancing particles simulation
//!
//! See README.md for additional information.

extern crate minutiae;
extern crate noise;
extern crate uuid;

extern {
    pub fn canvas_render(ptr: *const u8);
}

use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialEntityIterator, SerialGridIterator};
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use minutiae::driver::middleware::MinDelay;
use uuid::Uuid;

mod engine;
use engine::DancerEngine;

const UNIVERSE_SIZE: usize = 800;
const PARTICLE_COUNT: usize = 2000;

// minutiae type definitions

#[derive(Clone)]
// These hold the hidden noise values that determine the behavior of the entities.
struct CS {
    noise_val_1: f32,
    noise_val_2: f32,
}

impl CS {
    fn get_color(&self) -> [u8; 4] {
        unimplemented!();
    }
}

impl CellState for CS {}

#[derive(Clone)]
struct ES {}

#[derive(Clone, Copy, Default)]
struct MES {}
impl MutEntityState for MES {}

enum CA {}

impl CellAction<CS> for CA {}

enum EA {}
impl EntityAction<CS, ES> for EA {}

impl EntityState<CS> for ES {}

// dummy function until `cell_mutator` is deprecated entirely
fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

struct WG;
impl Generator<CS, ES, MES, CA, EA> for WG {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        let base_cell = Cell{
            state: CS {
                noise_val_1: 0.0,
                noise_val_2: 0.0
            }
        };
        let cells = vec![base_cell; conf.size * conf.size];

        let entities = Vec::new();
        // randomly distribute some starter entities into the universe.

        (cells, entities)
    }
}

fn entity_driver(
    universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {
    unimplemented!();
}

fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    if entity_indexes.len() == 0 { [0, 0, 0, 1] } else { cell.state.get_color() }
}

fn main() {
    let conf = UniverseConf {
        iter_cells: false,
        size: 800,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WG, cell_mutator, entity_driver);
    let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(DancerEngine);
    let driver = EmscriptenDriver;
    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
