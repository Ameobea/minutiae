use minutiae::prelude::*;
use minutiae::engine::parallel::ParallelEngine;

use entity_driver::our_entity_driver;
use super::*;

pub fn exec_actions(
    _universe: &mut Universe2D<CS, ES, MES>,
    _cell_actions: &[OwnedAction<CS, ES, CA, EA, usize>],
    _self_actions: &[OwnedAction<CS, ES, CA, EA, usize>],
    _entity_actions: &[OwnedAction<CS, ES, CA, EA, usize>],
) {

}

pub fn get_engine<'u>() -> impl Engine<
    CS, ES, MES, CA, EA, Universe2D<CS, ES, MES>
> {
    let engine: ParallelEngine<
        CS,
        ES,
        MES,
        CA,
        EA,
        usize,
        Vec<Cell<CS>>,
        Universe2D<CS, ES, MES>,
        _
    > = ParallelEngine::new(exec_actions, our_entity_driver);

    Box::new(engine)
}
