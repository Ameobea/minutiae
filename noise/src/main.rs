//! My goal for this is to play around with the noise-rs crate and create some noise with which to populate the universe of a
//! minutiae world.  We'll use 3D perlin noise and have the third coordinate correspond to the sequence number.

// TODO: implement middleware for closures that have the required `before_render`/`after_render` signature
// TODO: look into auto-implementing cell action/entity action for T since they don't have any requirements and possibly
//       implementing CA/EA for `()`
// TODO: Deprecate the entire cell mutator functionality in favor of entirely middleware-driven approaches

extern crate minutiae;
extern crate noise;

use minutiae::prelude::*;
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};
use noise::{Max, Billow, Constant, MultiFractal, NoiseModule, Seedable, Fbm, HybridMulti, Point2, Point3, Worley};

extern {
    pub fn canvas_render(ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 575;
const MULTIPLIER: f32 = /*0.013923431*/0.0912;

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
    fn step(&mut self, universe: &mut OurUniverse) {
        // no-op; all logic is handled by the middleware
        universe.seq += 1;
    }
}

/// given a buffer containing all of the cells in the universe, calculates values for each of them using
/// perlin noise and sets their states according to the result.
fn drive_noise(cells_buf: &mut [Cell<CS>], seq: usize, noise: &mut NoiseModule<Point3<f32>, Output=f32>) {
    let seq = seq as f32;
    for y in 0..UNIVERSE_SIZE {
        for x in 0..UNIVERSE_SIZE {
            // calculate noise value for current coordinate and sequence number
            let val = noise.get([x as f32 * MULTIPLIER, y as f32 * MULTIPLIER, seq * MULTIPLIER]);
            // set the cell's state equal to that value
            let index = get_index(x, y, UNIVERSE_SIZE);
            cells_buf[index].state = CS(val);
            // println!("{}", val);
        }
    }
}

/// Defines a middleware that sets the cell state of
struct NoiseStepper<N: NoiseModule<Point3<f32>, Output=f32>>(N);

impl<N: NoiseModule<Point3<f32>, Output=f32>> Middleware<CS, ES, MES, CA, EA, OurEngine> for NoiseStepper<N> {
    fn after_render(&mut self, universe: &mut OurUniverse) {
        drive_noise(&mut universe.cells, universe.seq, &mut self.0)
    }
}

fn calc_color(cell: &Cell<CS>, _: &[usize], _: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    let shade = (128 + (cell.state.0 * 128.0) as i16) as u8;
    [shade, shade, shade, 255]
}

struct WorldGenerator;
impl Generator<CS, ES, MES, CA, EA> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        // initialize blank universe
        (vec![Cell{state: CS(0.0)}; UNIVERSE_SIZE * UNIVERSE_SIZE], Vec::new())
    }
}

fn main() {
    let mut noise1: Fbm<f32> = Fbm::new();
    // .set_octaves(3)
        // .set_lacunarity(2.0)
        // .set_persistence(0.237f32);
    // let mut noise: HybridMulti<f32> = HybridMulti::new()
    let mut noise2: Worley<f32> = Worley::new()
        .set_seed(199919776);

    let constant = Constant::new(0.0f32);

    let mut noise3 = Max::new(noise1, noise2);

    // let multiplier: f32 = 10.1231;
    // let vals: Vec<f32> = (0..UNIVERSE_SIZE * UNIVERSE_SIZE).map(|i| {
    //     let (x, y) = get_coords(i, UNIVERSE_SIZE);
    //     noise.get([x as f32 * multiplier, y as f32 * multiplier])
    // }).collect();

    // initialize emscripten universe and start simulation
    let mut conf = UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let universe = Universe::new(conf, &mut WorldGenerator, |_, _| { None }, |_, _, _, _, _, _, _| {});
    let driver = EmscriptenDriver.init(universe, OurEngine, &mut [
        Box::new(NoiseStepper(noise3)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, calc_color, canvas_render))
    ]);
}
