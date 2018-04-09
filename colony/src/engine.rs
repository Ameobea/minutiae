use std::fmt::Debug;

use minutiae::prelude::*;
use minutiae::engine::parallel::ParallelEngine;
use minutiae::server::Tys;
use minutiae::universe::{CellContainer, ContiguousUniverse};
use uuid::Uuid;

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
    CS,
    ES,
    MES,
    CA,
    EA,
    Universe2D<CS, ES, MES>,
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

    box engine
}

pub fn get_custom_engine<
    CS: CellState + Send + Debug + 'static,
    ES: EntityState<CS> + Send + Debug,
    MES: MutEntityState + Send,
    CA: CellAction<CS> + Send + Debug,
    EA: EntityAction<CS, ES> + Send + Debug,
    I: Ord + Copy + Send + 'static,
    CC: CellContainer<CS, I> + Send + 'static,
    U: Universe<CS, ES, MES, Coord=I> + ContiguousUniverse<CS, ES, MES, I, CC>,
    F: Fn(
        &mut U,
        &[OwnedAction<CS, ES, CA, EA, I>],
        &[OwnedAction<CS, ES, CA, EA, I>],
        &[OwnedAction<CS, ES, CA, EA, I>],
    ),
>(
    exec_actions: F,
    entity_driver: fn(
        source_universe_index: I,
        entity: &Entity<CS, ES, MES>,
        entities: &EntityContainer<CS, ES, MES, I>,
        cells: &[Cell<CS>],
        cell_action_executor: &mut FnMut(CA, I),
        self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
        entity_action_executor: &mut FnMut(EA, usize, Uuid)
    )
) -> impl Engine<CS, ES, MES, CA, EA, U> {
    let engine: ParallelEngine<
        CS, ES, MES, CA, EA, I, CC, U, F
    > = ParallelEngine::new(exec_actions, entity_driver);

    box engine
}
