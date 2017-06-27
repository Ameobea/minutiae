//! Defines the behavior of the engine that regulates the bahavior of the universe as a whole.

use super::*;

pub struct DancerEngine;

fn exec_cell_action(action: &OwnedAction<CS, ES, CA, EA>, cells: &mut[Cell<CS>], entities: &mut EntityContainer<CS, ES, MES>) {
    unimplemented!();
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>, entities: &mut EntityContainer<CS, ES, MES>) {
    unimplemented!();
}

fn exec_entity_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!();
}

impl SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>> for DancerEngine {
    fn iter_cells(&self, cells: &[Cell<CS>]) -> SerialGridIterator {
        SerialGridIterator::new(UNIVERSE_SIZE)
    }

    fn iter_entities(&self, entities: &[Vec<Entity<CS, ES, MES>>]) -> SerialEntityIterator<CS, ES> {
        SerialEntityIterator::new(UNIVERSE_SIZE)
    }

    fn exec_actions(
        &self, universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
        self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
    ) {
        for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe.cells, &mut universe.entities); }
        for self_action in self_actions { exec_self_action(self_action, &mut universe.entities); }
        for entity_action in entity_actions { exec_entity_action(entity_action); }
    }
}
