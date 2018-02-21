#![feature(box_syntax, conservative_impl_trait, integer_atomics, test)]

extern crate minutiae;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::server::ColorServer;
use minutiae::server::Server;
use minutiae::util::Color;

mod engine;
mod sparse_universe;
mod world_generator;

use engine::ColonyEngine;
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

#[derive(Clone, Serialize, Deserialize)]
pub enum ES {

}

#[derive(Clone, Default, Copy, Serialize, Deserialize)]
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
    let engine = ColonyEngine;

    let colorserver = ColorServer::new(
        color_calculator,
        |start, end| UniverseIterator::new(start, end),
        P2D { x: 0, y: 0 },
        P2D { x: 500, y: 500 }
    );

    driver.init(universe, engine, &mut [
        box Server::new("localhost", colorserver, Arc::new(AtomicU32::new(0))),
    ]);
}
