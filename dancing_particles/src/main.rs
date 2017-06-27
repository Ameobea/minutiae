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
use interop::GenType;
mod noise_engine;
use noise_engine::*;

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
        let avg_color = (self.noise_val_1 + self.noise_val_2) / 2.;
        let hue = (avg_color * 360.0) + 180.0;
        let hsv_color = Hsv::new(hue.into(), 1.0, 1.0);
        let rgb_color = Rgb::from_hsv(hsv_color);
        [(rgb_color.red * 255.0) as u8, (rgb_color.green * 255.0) as u8, (rgb_color.blue * 255.0) as u8, 255]
    }
}

impl CellState for CS {}

#[derive(Clone)]
pub struct ES {secondary: bool}

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
        let base_cell = Cell{
            state: CS {
                noise_val_1: 0.0,
                noise_val_2: 0.0
            }
        };
        let cells = vec![base_cell; conf.size * conf.size];

        let mut rng = PcgRng::new_unseeded();

        let mut entities = vec![Vec::new(); UNIVERSE_SIZE * UNIVERSE_SIZE];
        // randomly distribute some starter entities into the universe.
        let mut spawned_particles = 0;
        while spawned_particles < PARTICLE_COUNT {
            let index: usize = rng.gen_range(0, UNIVERSE_SIZE * UNIVERSE_SIZE);
            if entities[index].len() == 0 {
                entities[index].push(Entity::new(ES {secondary: rng.gen()}, MES {}));
                spawned_particles += 1;
            }
        }

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
    let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);
    // look around us and find the cell with the highest value for the dimension we're following and move towards it.
    let mut best_coord: (usize, f32) = (universe_index, -100000000000.0);
    for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
        let index = get_index(x, y, UNIVERSE_SIZE);
        let val = if !entity.state.secondary { cells[index].state.noise_val_1 } else { cells[index].state.noise_val_2 };
        if val > best_coord.1 {
            best_coord = (index, val);
        }
    }

    if best_coord.0 != universe_index {
        let (cell_x, cell_y) = get_coords(best_coord.0, UNIVERSE_SIZE);
        let our_x_offset = if cur_x > cell_x { -1 } else if cur_x == cell_x { 0 } else { 1 };
        let our_y_offset = if cur_y > cell_y { -1 } else if cur_y == cell_y { 0 } else { 1 };
        let self_action = SelfAction::Translate(our_x_offset, our_y_offset);
        return self_action_executor(self_action);
    }
}

fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    if entity_indexes.len() == 0 { cell.state.get_color() } else {
        if ! unsafe { entity_container.get(entity_indexes[0]).state.secondary } { [255, 0, 0, 255] } else { [0, 255, 0, 255] }
    }
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
    let mut conf = NoiseEngine::default();
    conf.generator_type = GenType::SuperSimplex;
    conf.canvas_size = UNIVERSE_SIZE;
    // conf.octaves = 1;
    conf.speed = SPEED;
    conf.zoom = ZOOM;
    conf.needs_new_noise_gen = true;
    conf.needs_update = true;
    let noise_middleware = NoiseMiddleware {
        conf: Box::new(conf),
        noise_engine: Box::into_raw(Box::new(noise_module)) as *mut c_void,
    };

    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(noise_middleware),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
