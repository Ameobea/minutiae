//! The engine defines the way that the simulation handles events submitted by entities.  It's in charge of
//! resolving conflicts, verifying their validity, and applying the actions on the state.

use super::*;

pub struct OurEngine {}

fn exec_cell_action(
    action: &OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>,
    universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>
) {
    match action.action {
        Action::CellAction{universe_index, ..} => {
            let (cell_x, cell_y) = get_coords(universe_index, UNIVERSE_SIZE);
            if cell_x < UNIVERSE_SIZE && cell_y < UNIVERSE_SIZE {
                let cell_index = get_index(cell_x as usize, cell_y as usize, UNIVERSE_SIZE);
                // consume the food by replacing it with water
                universe.cells[cell_index].state = OurCellState::Water;
            }
        },
        _ => unreachable!(),
    }
}

fn exec_self_action(
    action: &OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>,
    universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>
) {
    match action.action {
        Action::SelfAction(ref self_action) => {
            let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
            match *self_action {
                SelfAction::Translate(x_offset, y_offset) => {
                    // this function will return early if the entity has been deleted
                    let universe_index = match universe.entities.get_verify(entity_index, entity_uuid) {
                        Some((_, universe_index)) => universe_index,
                        None => { return; }, // entity has been deleted, so do nothing.
                    };

                    // if this is the entity that we're looking for, check to see if the requested move is in bounds
                    let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);
                    let new_x = cur_x as isize + x_offset;
                    let new_y = cur_y as isize + y_offset;

                    // verify that the supplied desination coordinates are in bounds
                    // TODO: verify that the supplied destination coordinates are within ruled bounds of destination
                    if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                        let dst_universe_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);
                        universe.entities.move_entity(entity_index, dst_universe_index);
                    }
                },
                SelfAction::Custom(OurEntityAction::SetVector(x, y)) => {
                    // locate the entity that dispatched this request and mutate its state with the supplied value
                    // our implementation asserts that the entity will not have moved before this takes place, so
                    // a simple search is sufficient to locate it.
                    let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
                    if let Some((entity, _)) = universe.entities.get_verify_mut(entity_index, entity_uuid) {
                        match entity.state {
                            OurEntityState::Predator{ref mut direction, ..} => {
                                *direction = Some((x, y));
                            },
                            _ => unreachable!(),
                        }
                    }
                },
                _ => unimplemented!(),
            }
        },
        _ => unreachable!(),
    }
}

fn exec_entity_action(
    action: &OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>,
    universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>
) {
    match action.action {
        Action::EntityAction{action: ref entity_action, target_entity_index, target_uuid} => {
            match *entity_action {
                OurEntityAction::EatFish => {
                    // check to see if the shark (source entity) is still alive
                    let (src_entity_index, src_uuid) = (action.source_entity_index, action.source_uuid);

                    let src_universe_index = match universe.entities.get_verify_mut(src_entity_index, src_uuid) {
                        Some((_, src_universe_index)) => src_universe_index,
                        None => { return; },
                    };

                    let dst_universe_index = match universe.entities.get_verify_mut(target_entity_index, target_uuid) {
                        Some((_, dst_universe_index)) => dst_universe_index,
                        None => { return; }, // fish has been deleted so abort
                    };

                    // bail out early if the fish has moved out of range
                    let (src_x, src_y) = get_coords(src_universe_index, UNIVERSE_SIZE);
                    let (entity_x, entity_y) = get_coords(dst_universe_index, UNIVERSE_SIZE);
                    if manhattan_distance(src_x, src_y, entity_x, entity_y) > 1 {
                        return;
                    } else {
                        // I eat the fish
                        let eaten_fish = universe.entities.remove(target_entity_index);
                        debug_assert_eq!(eaten_fish.uuid, target_uuid);
                    }

                    // increment the food value of the source entity
                    match *unsafe { &mut universe.entities.get_mut(src_entity_index).state } {
                        OurEntityState::Predator{ref mut food, ..} => { *food += 1 },
                        _ => unreachable!(),
                    }
                },
                OurEntityAction::MakeBaby => unimplemented!(),
                OurEntityAction::SetVector(_, _) => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}

pub fn exec_actions(
    universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>,
    cell_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
    self_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
    entity_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
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

#[cfg(target_os = "emscripten")]
impl SerialEngine<
    OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction,
    SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>
> for OurEngine {
    fn iter_cells(&self, cells: &[Cell<OurCellState>]) -> SerialGridIterator {
        SerialGridIterator::new(cells.len())
    }

    fn iter_entities<'a>(
        &self, entities: &'a [Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>]
    ) -> SerialEntityIterator<OurCellState, OurEntityState> {
        SerialEntityIterator::new(entities.len())
    }

    fn exec_actions(
        &self,
        universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>,
        cell_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
        self_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
        entity_actions: &[OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>],
    ) {
        exec_actions(universe, cell_actions, self_actions, entity_actions);
    }
}
