//! Dancing particles simulation
//!
//! See README.md for additional information.

extern crate minutiae;
extern crate noise;
extern crate palette;
extern crate pcg;
extern crate rand;
extern crate uuid;

extern {
    pub fn canvas_render(ptr: *const u8);
}

use std::os::raw::c_void;

use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialEntityIterator, SerialGridIterator};
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use minutiae::driver::middleware::MinDelay;
use noise::*;
use palette::{FromColor, Hsv, Rgb};
use pcg::PcgRng;
use rand::Rng;
use uuid::Uuid;

mod engine;
use engine::DancerEngine;
mod interop;

const UNIVERSE_SIZE: usize = 500;
const PARTICLE_COUNT: usize = 20000;
const VIEW_DISTANCE: usize = 1;
const SPEED: f32 = 0.00758;
const ZOOM: f32 = 0.00132312;

// minutiae type definitions

#[derive(Clone)]
// These hold the hidden noise values that determine the behavior of the entities.
pub struct CS {
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
pub struct ES {}

#[derive(Clone, Copy, Default)]
pub struct MES {}
impl MutEntityState for MES {}

pub enum CA {}

impl CellAction<CS> for CA {}

pub enum EA {}
impl EntityAction<CS, ES> for EA {}

impl EntityState<CS> for ES {}

// dummy function until `cell_mutator` is deprecated entirely
pub fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

struct WG;
impl Generator<CS, ES, MES, CA, EA> for WG {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        unimplemented!();
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
    let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);
    unimplemented!();
}

fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    unimplemented!();
}

fn main() {
    let conf = UniverseConf {
        iter_cells: false,
        size: UNIVERSE_SIZE,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WG, cell_mutator, entity_driver);
    let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(DancerEngine);
    let driver = EmscriptenDriver;

    let noise_module: SuperSimplex = SuperSimplex::new();

    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
