//! Defines functions that define the behavior of the engine.

use minutiae::prelude::{Universe, OwnedAction};

use super::*;

pub fn exec_actions(
    universe: &mut Universe2D<CS, ES, MES>,
    self_actions: &[OwnedAction<CS, ES, CA, EA>],
    cell_actions: &[OwnedAction<CS, ES, CA, EA>],
    entity_actions: &[OwnedAction<CS, ES, CA, EA>]
) {
    // process actions in order of cell actions, then self actions, and finally entity actions
    for cell_action in cell_actions {
        exec_cell_action(cell_action, universe);
    }

    for self_action in self_actions {
        exec_self_action(self_action, universe);
    }

    for entity_action in entity_actions {
        exec_entity_action(entity_action, universe);
    }
}

pub fn exec_cell_action(
    action: &OwnedAction<CS, ES, CA, EA>,
    universe: &mut Universe2D<CS, ES, MES>
) {
    unimplemented!(); // TODO
}

pub fn exec_self_action(
    action: &OwnedAction<CS, ES, CA, EA>,
    universe: &mut Universe2D<CS, ES, MES>
) {
    unimplemented!(); // TODO
}

pub fn exec_entity_action(
    action: &OwnedAction<CS, ES, CA, EA>,
    universe: &mut Universe2D<CS, ES, MES>
) {
    unimplemented!(); // TODO
}
