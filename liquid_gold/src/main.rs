//! Dancing particles simulation
//!
//! See README.md for additional information.

#![feature(advanced_slice_patterns, slice_patterns)]

extern crate minutiae;
extern crate noise;
extern crate palette;
extern crate pcg;
extern crate rand;
extern crate uuid;

extern crate libcomposition;

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
mod entity_driver;
use entity_driver::entity_driver;
mod interop;
mod composition_def;
use composition_def::COMPOSITION_DEF;

const UNIVERSE_SIZE: usize = 500;
const PARTICLE_COUNT: usize = 20000;
const VIEW_DISTANCE: usize = 1;
const SPEED: f32 = 0.00758;
const ZOOM: f32 = 0.00132312;

fn rgb_to_array(rgb: &Rgb) -> [u8; 4] {
    [(rgb.red * 255.) as u8, (rgb.green * 255.) as u8, (rgb.blue * 255.) as u8, 255]
}

/// A vector (in the physics sense) determining the X and Y velocity of a particle.
/// The values correspond to cells per tick.
#[derive(Clone)]
pub struct Velocity {
    x: f32,
    y: f32,
    // These offsets are a way to bridge the discrete universe with our floating-point velocities.
    // They represent how far from the center of a particular cell we are.  If our coordinates are (10, 10)
    // and our offets are (-0.2, 0.5), our effective floating-point coordinates are (9.8, 10.5).
    x_offset: f32,
    y_offset: f32,
}

// minutiae type definitions

#[derive(Clone)]
// These hold the hidden noise values that determine the behavior of the entities.
pub struct CS {
    noise_val_1: f32,
    noise_val_2: f32,
}

impl CellState for CS {}

#[derive(Clone)]
pub enum ES {
    Builder,
    Gem(f32), // f32 is a number from -1 to 1 representing the color of this gem particle
    Dust{ shade: f32, velocity: Velocity },
}

impl ES {
    pub fn get_base_color(&self) -> Rgb {
        match self {
            &ES::Builder => Rgb::new_u8(0, 0, 0),
            &ES::Gem(color) => {
                let hue = (color * 360.0) + 180.0;
                let hsv_color = Hsv::new(hue.into(), 1.0, 1.0);
                Rgb::from_hsv(hsv_color)
            },
            &ES::Dust{ shade, ref velocity } => unimplemented!(), // TODO
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct MES {}
impl MutEntityState for MES {}

pub enum CA {

}

impl CellAction<CS> for CA {}

pub enum EA {
    /// Update the velocity of the entity with the given vector and translate according to the result.
    UpdateVelocities { x: f32, y: f32 }
}
impl EntityAction<CS, ES> for EA {}

impl EntityState<CS> for ES {}

// dummy function until `cell_mutator` is deprecated entirely
pub fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

struct WG;
impl Generator<CS, ES, MES, CA, EA> for WG {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        // start with an empty universe
        let cells = vec![Cell { state: CS { noise_val_1: 0.0, noise_val_2: 0.0 }}; UNIVERSE_SIZE * UNIVERSE_SIZE];
        let entities = vec![vec![]; UNIVERSE_SIZE];

        (cells, entities)
    }
}

/// Given a base color and an offset value as a f32, changes it according to the degree of offset.
fn offset_color(base: &Rgb, noise: f32) -> Rgb {
    unimplemented!();
}

fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    let rgb_color = match entity_indexes {
        &[] => {
            offset_color(&Rgb::new_u8(3, 3, 3), cell.state.noise_val_1)
        },
        &[.., last_index] => {
            let entity = unsafe { entity_container.get(last_index) };
            let base_color = entity.state.get_base_color();
            offset_color(&base_color, cell.state.noise_val_2)
        }
    };

    rgb_to_array(&rgb_color)
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

    // construct the noise module from the definition string
    let (colorfn, noise_module) = libcomposition::util::build_tree_from_def(COMPOSITION_DEF).unwrap();

    driver.init(universe, engine, &mut [
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
