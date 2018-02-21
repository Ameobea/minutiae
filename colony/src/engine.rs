use minutiae::prelude::*;
use minutiae::engine::parallel::ParallelEngine;

use entity_driver::our_entity_driver;
use sparse_universe::{P2D, Sparse2DUniverse};
use world_generator::WorldGenerator;
use super::*;

pub fn exec_actions(
    universe: &mut Sparse2DUniverse<CS, ES, MES, WorldGenerator>,
    cell_actions: &[OwnedAction<CS, ES, CA, EA, P2D>],
    self_actions: &[OwnedAction<CS, ES, CA, EA, P2D>],
    entity_actions: &[OwnedAction<CS, ES, CA, EA, P2D>],
) {

}

pub fn get_engine<'u>() -> impl Engine<CS, ES, MES, CA, EA, Sparse2DUniverse<CS, ES, MES, WorldGenerator>> {
    let engine: ParallelEngine<
        CS,
        ES,
        MES,
        CA,
        EA,
        P2D,
        Sparse2DUniverse<CS, ES, MES, WorldGenerator>,
        Sparse2DUniverse<CS, ES, MES, WorldGenerator>,
    > = ParallelEngine::new(box exec_actions, our_entity_driver);

    box engine
}
