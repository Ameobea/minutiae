//! Defines the behavior of the engine that regulates the bahavior of the universe as a whole.

use super::*;

pub struct DancerEngine;

fn exec_cell_action(action: &OwnedAction<CS, ES, CA, EA>, cells: &mut[Cell<CS>], entities: &mut EntityContainer<CS, ES, MES>) {
    unimplemented!();
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>, entities: &mut EntityContainer<CS, ES, MES>) {
    match action.action {
        Action::SelfAction(ref self_action) => {
            let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
            match *self_action {
                SelfAction::Translate(x_offset, y_offset) => {
                    // this function will return early if the entity has been deleted
                    let universe_index = match entities.get_verify(entity_index, entity_uuid) {
                        Some((_, universe_index)) => universe_index,
                        None => { return; }, // entity has been deleted, so do nothing.
                    };

                    // if this is the entity that we're looking for, check to see if the requested move is in bounds
                    let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);
                    let new_x = cur_x as isize + x_offset;
                    let new_y = cur_y as isize + y_offset;
                    let dst_universe_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);

                    // make sure there are no entities where we're trying to move
                    if entities.get_entities_at(dst_universe_index).len() > 0 {
                        return;
                    }

                    // verify that the supplied desination coordinates are in bounds
                    // TODO: verify that the supplied destination coordinates are within ruled bounds of destination
                    if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                        entities.move_entity(entity_index, dst_universe_index);
                    }
                },
                _ => unimplemented!(),
            }
        },
        _ => unreachable!(),
    }
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
