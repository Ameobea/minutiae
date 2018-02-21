#![feature(conservative_impl_trait, integer_atomics, test)]

extern crate minutiae;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;
extern crate uuid;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::driver::middleware::MinDelay;
use minutiae::server::ColorServer;
use minutiae::server::Server;
use minutiae::util::Color;

mod engine;
mod entity_driver;
mod sparse_universe;
mod world_generator;

use sparse_universe::{P2D, Sparse2DUniverse, UniverseIterator};
use world_generator::WorldGenerator;

pub const UNIVERSE_SIZE: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ES {

}

#[derive(Clone, Default, Copy, Serialize, Deserialize)]
pub struct MES {

}

impl MutEntityState for MES{}

impl EntityState<CS> for ES {}

#[derive(Clone, Debug)]
pub enum CA {

}

impl CellAction<CS> for CA {}

#[derive(Clone, Debug)]
pub enum EA {

}

impl EntityAction<CS, ES> for EA {}

fn color_calculator(
    cell: &Cell<CS>,
    entity_indexes: &[usize],
    entity_container: &EntityContainer<CS, ES, MES, P2D>
) -> Color {
    Color([0u8, 0u8, 0u8])
}

fn main() {
    let universe = sparse_universe::Sparse2DUniverse::new(WorldGenerator);
    let driver = BasicDriver;

    let engine = engine::get_engine();

    let colorserver = ColorServer::new(
        color_calculator,
        |start, end| UniverseIterator::new(start, end),
        P2D { x: 0, y: 0 },
        P2D { x: 500, y: 500 }
    );

    driver.init(universe, engine::get_engine(), &mut [
        Box::new(Server::new("0.0.0.0:7037", colorserver, Arc::new(AtomicU32::new(0)))),
        Box::new(MinDelay::from_tps(20.)),
    ]);
}
