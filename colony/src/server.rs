#![feature(integer_atomics)]

extern crate colony;
extern crate minutiae;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use colony::*;
use colony::world_generator::WorldGenerator;
use colony::sparse_universe::{P2D, Sparse2DUniverse};
use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::server::HybridServer;
use minutiae::server::Server;
use minutiae::server::Tys;
use minutiae::driver::middleware::MinDelay;

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
    let driver = BasicDriver;

    let server: HybridServer<ColonyTys> = HybridServer::new(colony_event_generator);

    driver.init(universe, engine::get_engine(), &mut [
        Box::new(Server::new("0.0.0.0:7037", server, Arc::new(AtomicU32::new(0)))),
        Box::new(MinDelay::from_tps(20.)),
    ]);
}
