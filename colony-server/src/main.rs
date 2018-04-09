#![feature(integer_atomics)]

extern crate colony;
extern crate minutiae;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use colony::*;
use colony::engine::{exec_actions, get_custom_engine};
use colony::entity_driver::our_entity_driver;
use colony::sparse_universe::{CellGeneratorWrapper, P2D};
use colony::world_generator::{WorldGenerator};
use minutiae::prelude::*;
use minutiae::driver::BasicDriver;
use minutiae::server::HybridServer;
use minutiae::server::Server;
use minutiae::server::Tys;
use minutiae::driver::middleware::MinDelay;
use minutiae::universe::{Universe2D, Universe2DConf};

fn colony_event_generator(
    _universe: &mut <ColonyTys as Tys>::U,
    seq: u32,
    _cell_actions: &[OwnedAction<CS, ES, CA, EA, usize>],
    _self_actions: &[OwnedAction<CS, ES, CA, EA, usize>],
    _entity_actions: &[OwnedAction<CS, ES, CA, EA, usize>]
) -> Option<Vec<ColonyEvent>> {
    // let tick = seq % 100;
    // if tick != 0 {
    //     return None;
    // }

    let color = if seq % 2 == 0 { [150, 222, 14, 255] } else { [133, 133, 12, 255] };
    let coord = P2D { x: (15 + (seq % 700)) as usize, y: (15 + (seq % 700)) as usize};
    let evt = ColonyEvent::Splat(coord, color);
    Some(vec![evt])
}

pub fn main() {
    let mut wrapper: CellGeneratorWrapper<_, _, _, _, WorldGenerator> = CellGeneratorWrapper::new();
    let universe = Universe2D::new(Universe2DConf { size: 800 }, &mut wrapper);
    let driver = BasicDriver;

    let (wrapped_action_executor, server_logic) = HybridServer::<CustomClientMessage, ColonyTys>::hook_handler(
        exec_actions,
        colony_event_generator,
        custom_event_handler
    );
    let engine = get_custom_engine::<CS, ES, MES, CA, EA, usize, _, _, _>(wrapped_action_executor, our_entity_driver);

    driver.init(universe, engine, &mut [
        Box::new(Server::new("0.0.0.0:7037", server_logic, Arc::new(AtomicU32::new(0)))),
        Box::new(MinDelay::from_tps(20.)),
    ]);
}
