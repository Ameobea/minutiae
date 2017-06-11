//! My goal for this is to play around with the noise-rs crate and create some noise with which to populate the universe of a
//! minutiae world.  We'll use 3D perlin noise and have the third coordinate correspond to the sequence number.

// TODO: implement middleware for closures that have the required `before_render`/`after_render` signature
// TODO: look into auto-implementing cell action/entity action for T since they don't have any requirements and possibly
//       implementing CA/EA for `()`
// TODO: Deprecate the entire cell mutator functionality in favor of entirely middleware-driven approaches

#![allow(unused_variables, dead_code)]

#![feature(alloc)]

extern crate alloc;
#[macro_use]
extern crate lazy_static;
extern crate minutiae;
extern crate noise;
extern crate palette;

use std::{i32, u16, u32};
use std::mem::transmute;

use minutiae::prelude::*;
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use minutiae::driver::BasicDriver;
use noise::{
    Blend, Clamp, Max, Billow, Constant, MultiFractal, NoiseModule, OpenSimplex, Seedable,
    Fbm, HybridMulti, Point2, Point3, RidgedMulti, SuperSimplex, Value, Worley
};
use palette::{FromColor, Hsv, Rgb, RgbHue};

extern {
    pub fn canvas_render(ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 575;
const ZOOM: f32 = 0.0132312;
const TIME_SCALE: f32 = 0.00758;

lazy_static!{
    static ref NOISE_1: Fbm<f32> = Fbm::new();
    static ref NOISE_2: Worley<f32> = Worley::new();
    static ref NOISE_3: OpenSimplex = OpenSimplex::new();
    static ref NOISE_4: Billow<f32> = Billow::new();
    static ref NOISE_5: HybridMulti<f32> = HybridMulti::new();
    static ref NOISE_6: SuperSimplex = SuperSimplex::new();
    static ref NOISE_7: Value = Value::new();
    static ref NOISE_8: RidgedMulti<f32> = RidgedMulti::new();
}

struct NoiseUpdater;

// minutiae custom type declarations
#[derive(Clone)]
struct CS(f32);
impl CellState for CS {}

#[derive(Clone)]
struct ES;
impl EntityState<CS> for ES {}

#[derive(Clone, Default)]
struct MES;
impl MutEntityState for MES {}

struct CA;
impl CellAction<CS> for CA {}

struct EA;
impl EntityAction<CS, ES> for EA {}

type OurUniverse = Universe<CS, ES, MES, CA, EA>;

struct OurEngine;
impl Engine<CS, ES, MES, CA, EA> for OurEngine {
    #[allow(unused_variables)]
    fn step(&mut self, universe: &mut OurUniverse) {
        // no-op; all logic is handled by the middleware
        universe.seq += 1;
    }
}

/// given a buffer containing all of the cells in the universe, calculates values for each of them using
/// perlin noise and sets their states according to the result.
fn drive_noise(cells_buf: &mut [Cell<CS>], seq: usize, noise: &mut NoiseModule<Point3<f32>, f32>) {
    let fseq = seq as f32;
    for y in 0..UNIVERSE_SIZE {
        for x in 0..UNIVERSE_SIZE {
            // calculate noise value for current coordinate and sequence number
            let val = noise.get([x as f32 * ZOOM, y as f32 * ZOOM, fseq * TIME_SCALE]);
            // set the cell's state equal to that value
            let index = get_index(x, y, UNIVERSE_SIZE);
            cells_buf[index].state = CS(val);
            // println!("{}", val);
        }
    }
}

/// Defines a middleware that sets the cell state of
struct NoiseStepper<N: NoiseModule<Point3<f32>, f32>>(N);

impl<N: NoiseModule<Point3<f32>, f32>> Middleware<CS, ES, MES, CA, EA, OurEngine> for NoiseStepper<N> {
    fn after_render(&mut self, universe: &mut OurUniverse) {
        drive_noise(&mut universe.cells, universe.seq, &mut self.0)
    }
}

// ZOOM = 0.00000092312
fn calc_color1(cell: &Cell<CS>, _: &[usize], _: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    // let shade = ((128.0 + (cell.state.0 * 128.0)) * (i32::MAX as f32)) as u32;
    // unsafe { ::std::mem::transmute::<_, _>(shade) }
    let mut buf: [u8; 4] = unsafe { transmute(cell.state.0) };
    buf[3] = 255;
    let mut buf2: u32 = unsafe { transmute(buf) };
    buf2 = buf2 ^ 0b101010101010101010101010;
    unsafe { transmute(buf2) }
}

// ZOOM = 0.00092312
fn calc_color(cell: &Cell<CS>, _: &[usize], _: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    // println!("{}", cell.state.0);
    // assert!(cell.state.0 <= 1.0 && cell.state.0 >= -01.0);

    // normalize into range from -180 to 180
    let mut hue = (cell.state.0 * 360.0) + 180.0;
    // hue = (hue * 0.5) + 180.0;
    let hsv_color = Hsv::new(hue.into(), 1.0, 1.0);
    let rgb_color = Rgb::from_hsv(hsv_color);
    [(rgb_color.red * 255.0) as u8, (rgb_color.green * 255.0) as u8, (rgb_color.blue * 255.0) as u8, 255]
}

struct WorldGenerator;
impl Generator<CS, ES, MES, CA, EA> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        // initialize blank universe
        (vec![Cell{state: CS(0.0)}; UNIVERSE_SIZE * UNIVERSE_SIZE], Vec::new())
    }
}

fn main() {
    let noise6: Blend<Point3<f32>, f32> = Blend::new(&*NOISE_1, &*NOISE_4, &*NOISE_5);

    // initialize emscripten universe and start simulation
    let mut conf = UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let universe = Universe::new(conf, &mut WorldGenerator, |_, _| { None }, |_, _, _, _, _, _, _| {});
    EmscriptenDriver.init(universe, OurEngine, &mut [
        Box::new(NoiseStepper(&*NOISE_8)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, calc_color, canvas_render))
    ]);
}
