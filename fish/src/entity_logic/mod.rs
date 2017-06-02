//! This module holds code that determines the behavior of entities.  It matches the state of each entity
//! to determine what kind it is (fish, predator, etc.) and looks at the cells ane entities around it to
//! determine what actions to take.

use super::*;

mod fish;
pub use self::fish::fish_driver;
mod predator;
pub use self::predator::predator_driver;

/// This function determines the core logic of the simulation.  Every entity evaluates this function every tick of the
/// simulation.  Actions are sent to the various executors and dispatched in batch after all entities have submitted them.
pub fn our_entity_driver(
    source_universe_index: usize,
    entity: &Entity<OurCellState, OurEntityState, OurMutEntityState>,
    entities: &EntityContainer<OurCellState, OurEntityState, OurMutEntityState>,
    cells: &[Cell<OurCellState>],
    cell_action_executor: &mut FnMut(OurCellAction, usize),
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>),
    entity_action_executor: &mut FnMut(OurEntityAction, usize, Uuid)
) {
    match entity.state {
        OurEntityState::Fish{..} => {
            fish_driver(source_universe_index, entity, entities, cells, cell_action_executor, self_action_executor);
        },
        OurEntityState::Predator{direction, ..} => {
            predator_driver(direction, source_universe_index, entity, entities, self_action_executor, entity_action_executor);
        }
    }
}
