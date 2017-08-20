//! Volumetric rendering of noise functions

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate minutiae;
extern crate noise;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use std::ffi::CString;
use std::os::raw::c_char;

use minutiae::prelude::*;
use minutiae::emscripten::EmscriptenDriver;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialGridIterator, SerialEntityIterator};
use noise::Billow;

extern {
    /// Invokes the external JS function to pass this buffer to WebGL and render it
    pub fn buf_render(ptr: *const f32);
    /// Direct line to `console.log` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_debug(msg: *const c_char);
    /// Direct line to `console.error` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_error(msg: *const c_char);
}

mod buf3d_middleware;
use buf3d_middleware::*;
mod engine;
use engine::*;
mod entity_driver;
use entity_driver::*;
mod noise_middleware;
use noise_middleware::NoiseStepper;

/// Wrapper around the JS error function that accepts a Rust `&str`.
pub fn error(msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { js_error(c_str.as_ptr()) };
}

const UNIVERSE_SIZE: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CS(Vec<f32>); // Psuedo-3d
impl CellState for CS {}
impl BufColumn for CS {
    fn get_col(&self) -> &[f32] { &self.0 } // nothing to compute since we're just storing `f32`s in the backend as well.
    fn get_col_mut(&mut self) -> &mut [f32] { &mut self.0 }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ES {
    Unit
}
impl EntityState<CS> for ES {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MES {
}
impl Default for MES { fn default() -> Self { MES {  } } }
impl MutEntityState for MES {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CA {
}

impl CellAction<CS> for CA {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EA {
    ClearMessengerState,
}
impl EntityAction<CS, ES> for EA {}

struct OurEngine;
impl SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>> for OurEngine {
    fn iter_cells(&self, cells: &[Cell<CS>]) -> SerialGridIterator {
        SerialGridIterator::new(UNIVERSE_SIZE)
    }

    fn iter_entities(&self, entities: &[Vec<Entity<CS, ES, MES>>]) -> SerialEntityIterator<CS, ES> {
        SerialEntityIterator::new(UNIVERSE_SIZE)
    }

    fn exec_actions(
        &self, mut universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
        self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
    ) {
        for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe); }
        for self_action in self_actions { exec_self_action(self_action, universe); }
        for entity_action in entity_actions { exec_entity_action(entity_action, universe); }
    }
}

pub struct WG;
impl Generator<CS, ES, MES, CA, EA> for WG {
    fn gen(&mut self, _: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        // create a blank universe to start off with
        (
            vec![Cell{ state: CS(vec![0.0f32; UNIVERSE_SIZE]) }; UNIVERSE_SIZE * UNIVERSE_SIZE],
            vec![vec![Entity::new(ES::Unit, MES::default())]]
        )
    }
}

// dummy function until `cell_mutator` is deprecated entirely
pub fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

pub fn main() {
    // set up the minutiae environment
    let conf = UniverseConf {
        iter_cells: false,
        size: UNIVERSE_SIZE,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WG, cell_mutator, entity_driver);
    let driver = EmscriptenDriver;
    let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(OurEngine);

    // create a noise generator to be used to populate the buffer
    let noise_gen = Billow::new();

    driver.init(universe, engine, &mut [
        // Box::new(MinDelay::from_tps(59.97)),
        Box::new(NoiseStepper::new(noise_gen, None)),
        Box::new(Buf3dWriter::new(UNIVERSE_SIZE, buf_render)),
    ]);
}
