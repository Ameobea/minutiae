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
extern crate num;

// extern crate libcomposition;

// extern {
//     pub fn canvas_render(ptr: *const u8);
// }

use std::os::raw::c_void;

use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::parallel::ParallelEngine;
use minutiae::engine::iterator::{SerialEntityIterator, SerialGridIterator};
use minutiae::driver::BasicDriver;
// use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use minutiae::driver::middleware::MinDelay;
use minutiae::driver::middleware::gif_renderer::GifRenderer;
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

const UNIVERSE_SIZE: usize = 300;
const VIEW_DISTANCE: usize = 10;
const SPEED: f32 = 0.00758;
const ZOOM: f32 = 0.00132312;
const DUST_COUNT: usize = 60000;
const BUILDER_COUNT: usize = 20000;
// normalized shade difference (-1.0, 1.0) * (VELOCITY_DISTANCE_FACTOR/distance) * VELOCITY_SCALE = velocity diff
const VELOCITY_DISTANCE_FACTOR: f32 = 0.2;
const VELOCITY_SCALE: f32 = 0.7;

fn rgb_to_array(rgb: &Rgb) -> [u8; 4] {
    [(rgb.red * 255.) as u8, (rgb.green * 255.) as u8, (rgb.blue * 255.) as u8, 255]
}

/// A vector (in the physics sense) determining the X and Y velocity of a particle.
/// The values correspond to cells per tick.
#[derive(Clone, Debug)]
pub struct Velocity {
    x: f32,
    y: f32,
    // These offsets are a way to bridge the discrete universe with our floating-point velocities.
    // They represent how far from the center of a particular cell we are.  If our coordinates are (9, 10)
    // and our offets are (0.8, 0.5), our effective floating-point coordinates are (9.8, 10.5).
    x_offset: f32,
    y_offset: f32,
}

impl Velocity {
    pub fn new(rng: &mut PcgRng) -> Self {
        Velocity {
            x: rng.gen_range(-0.2, 0.2),
            y: rng.gen_range(-0.2, 0.2),
            x_offset: 0.0,//rng.gen_range(-0.9, 0.9),
            y_offset: 0.0,//rng.gen_range(-0.9, 0.9),
        }
    }
}

// minutiae type definitions

#[derive(Clone, Debug)]
// These hold the hidden noise values that determine the behavior of the entities.
pub struct CS {
    noise_val_1: f32,
    noise_val_2: f32,
}

impl CellState for CS {}

#[derive(Clone, Debug)]
pub enum ES {
    Builder,
    Gem(f32), // f32 is a number from -1 to 1 representing the color of this gem particle
    Dust{ shade: f32, velocity: Velocity },
}

impl ES {
    pub fn get_base_color(&self) -> Rgb {
        match self {
            &ES::Builder => Rgb::new_u8(66, 244, 69),
            &ES::Gem(color) => {
                let hue = (color * 360.0) + 180.0;
                let hsv_color = Hsv::new(hue.into(), 1.0, 1.0);
                Rgb::from_hsv(hsv_color)
            },
            &ES::Dust{ shade, ref velocity } => {
                let hue = (shade * 180.0) + 90.0;
                let hsv_color = Hsv::new(hue.into(), 1.0, 1.0);
                Rgb::from_hsv(hsv_color)
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct MES([u8; 4]);
impl MutEntityState for MES {}

impl Default for MES {
    fn default() -> Self {
        MES([0; 4])
    }
}

impl MES {
    fn shift(&mut self, val: u8) {
        self.0[0] = self.0[1];
        self.0[1] = self.0[2];
        self.0[2] = self.0[3];
        self.0[3] = val;
    }

    fn shift_replace(self, val: u8) -> Self {
        let mut new = self.0.clone();
        new[0] = new[1];
        new[1] = new[2];
        new[2] = new[3];
        new[3] = val;

        MES(new)
    }
}

#[derive(Debug)]
pub enum CA {

}

impl CellAction<CS> for CA {}

#[derive(Debug)]
pub enum EA {
    /// Update the velocity of the entity with the given vector and translate according to the result.
    UpdateVelocities { x: f32, y: f32 },
    InvertShade,
}
impl EntityAction<CS, ES> for EA {}

impl EntityState<CS> for ES {}

// dummy function until `cell_mutator` is deprecated entirely
pub fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

/// Creates a new dust particle, initialized with random values
fn create_dust(rng: &mut PcgRng) -> Entity<CS, ES, MES> {
    let state = ES::Dust {
        shade: if rng.gen_range(0, 20) <= 2 { rng.gen_range(0.4, 1.0) } else { rng.gen_range(-1.0, -0.6) },
        velocity: Velocity::new(rng),
    };
    Entity::new(state, MES::default())
}

struct WG;
impl Generator<CS, ES, MES, CA, EA> for WG {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        // start with an empty universe
        let cells = vec![Cell { state: CS { noise_val_1: 0.0, noise_val_2: 0.0 }}; UNIVERSE_SIZE * UNIVERSE_SIZE];
        let mut entities: Vec<Vec<Entity<CS, ES, MES>>> = vec![Vec::new(); UNIVERSE_SIZE * UNIVERSE_SIZE];

        // create some fast PRNG using "true" RNG
        let mut true_rng = rand::thread_rng();
        let mut rng = PcgRng::new_unseeded();
        rng.set_stream(true_rng.gen());

        // randomly distribute some dust entities in the universe, initialized with random starting values
        for i in 0..DUST_COUNT {
            let (x, y) = (rng.gen_range(0, UNIVERSE_SIZE - 1), rng.gen_range(0, UNIVERSE_SIZE - 1));
            let universe_index = get_index(x, y, UNIVERSE_SIZE);

            if entities[universe_index].len() == 0 {
                let dust_particle = create_dust(&mut rng);
                entities[universe_index].push(dust_particle);
            }
        }

        // spawn some builders as well
        for i in 0..BUILDER_COUNT {
            let (x, y) = (rng.gen_range(0, UNIVERSE_SIZE - 1), rng.gen_range(0, UNIVERSE_SIZE - 1));
            let universe_index = get_index(x, y, UNIVERSE_SIZE);

            if entities[universe_index].len() == 0 {
                let builder = Entity::new(ES::Builder, MES::default());
                entities[universe_index].push(builder);
            }
        }

        (cells, entities)
    }
}

/// Given a base color and an offset value as a f32, changes it according to the degree of offset.
fn offset_color(base: &Rgb, noise: f32) -> Rgb {
    debug_assert!(noise >= -1.0 && noise <= 1.0);
    // unimplemented!();
    // TODO
    *base
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
        view_distance: VIEW_DISTANCE,
    };
    let universe = Universe::new(conf, &mut WG, cell_mutator, entity_driver);
    // let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(DancerEngine);
    let engine = Box::new(ParallelEngine::new(SerialGridIterator::new(UNIVERSE_SIZE), Box::new(engine::exec_actions), entity_driver));
    // let driver = EmscriptenDriver;
    let driver = BasicDriver;

    // construct the noise module from the definition string
    // let (colorfn, noise_module) = libcomposition::util::build_tree_from_def(COMPOSITION_DEF).unwrap();

    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(30.99)),
        // Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render)),
        Box::new(GifRenderer::new("./output.gif", UNIVERSE_SIZE, get_color)),
    ]);
}
