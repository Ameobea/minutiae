#![feature(box_syntax, test)]

extern crate minutiae;
extern crate test;

use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::server::ColorServer;

mod engine;
mod sparse_universe;
mod world_generator;

use engine::ColonyEngine;
use sparse_universe::Sparse2DUniverse;
use world_generator::WorldGenerator;

pub const UNIVERSE_SIZE: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CS {
    __placeholder,
    __placeholder2,
}

impl Default for CS {
    fn default() -> Self {
        CS::__placeholder
    }
}

impl CellState for CS {}

#[derive(Clone)]
pub enum ES {

}

#[derive(Clone, Default, Copy)]
pub struct MES {

}

impl MutEntityState for MES{}

impl EntityState<CS> for ES {}

pub enum CA {

}

impl CellAction<CS> for CA {}

pub enum EA {

}

impl EntityAction<CS, ES> for EA {}

fn color_calculator() {

}

fn main() {
    let universe = sparse_universe::Sparse2DUniverse::new(WorldGenerator, UNIVERSE_SIZE);
    let driver = BasicDriver;
    let engine = ColonyEngine;

    let middleware = &[
        box ColorServer::new(UNIVERSE_SIZE, color_calculator)
    ];

    driver.init(universe, engine, )
}
