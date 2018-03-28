#![feature(integer_atomics)]

extern crate colony;
extern crate minutiae;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use colony::*;
use colony::sparse_universe::{CellGenerator, Sparse2DUniverse, P2D};
use colony::world_generator::WorldGenerator;
use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::server::HybridServer;
use minutiae::server::Server;
use minutiae::server::Tys;
use minutiae::driver::middleware::MinDelay;
use minutiae::universe::{Universe2D, Universe2DConf, Into2DIndex};

fn colony_event_generator(
    universe: &mut <ColonyTys as Tys>::U,
    cell_actions: &[OwnedAction<CS, ES, CA, EA, P2D>],
    self_actions: &[OwnedAction<CS, ES, CA, EA, P2D>],
    entity_actions: &[OwnedAction<CS, ES, CA, EA, P2D>]
) -> Option<Vec<ColonyEvent>> {
    None
}

pub fn main() {
    let universe: Sparse2DUniverse<_, _, _, WorldGenerator> = Sparse2DUniverse::new();
    // let universe = Universe2D::new(Universe2DConf { size: 800 }, &mut OurWorldGenerator);
    let driver = BasicDriver;

    let server_logic: HybridServer<ColonyTys> = HybridServer::new(colony_event_generator);
    println!("Server message size: {}", ::std::mem::size_of::<<ColonyTys as Tys>::ServerMessage>());

    driver.init(universe, engine::get_engine(), &mut [
        Box::new(Server::new("0.0.0.0:7037", server_logic, Arc::new(AtomicU32::new(0)))),
        Box::new(MinDelay::from_tps(20.)),
    ]);
}
