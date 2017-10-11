//! Defines the behavior of the engine that regulates the bahavior of the universe as a whole.

use minutiae::action::Action;

use super::*;

pub struct DancerEngine;

fn exec_cell_action(action: &OwnedAction<CS, ES, CA, EA>, cells: &mut[Cell<CS>], entities: &mut EntityContainer<CS, ES, MES>) {
    unimplemented!();
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>, entities: &mut EntityContainer<CS, ES, MES>) {
    match action.action {
        Action::SelfAction(ref sa) => {
            let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
            match *sa {
                SelfAction::Translate(x_offset, y_offset) => {
                    let universe_index = match entities.get_verify(entity_index, entity_uuid) {
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
                        entities.move_entity(entity_index, dst_universe_index);
                    }
                },
                SelfAction::Custom(ref ea) => {
                    match ea {
                        &EA::UpdateVelocities { x: new_velocity_x, y: new_velocity_y } => {
                            let (mut entity, universe_index) = match entities.get_verify_mut (entity_index, entity_uuid) {
                                Some(d) => d,
                                None => { return; }, // entity has been deleted, so do nothing.
                            };

                            let velocity: &mut Velocity = match entity.state {
                                ES::Dust { ref mut velocity, .. } => velocity,
                                _ => panic!("Tried to dispatch an `UpdateVelocities` action on a non-dust particle!"),
                            };

                            // update the velocity with the new velocity vector
                            velocity.x += new_velocity_x;
                            velocity.y += new_velocity_y;

                            let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);

                            let &mut Velocity { x, y, x_offset, y_offset } = velocity;

                            // Calculate a translation based on our velocities.
                            let cur_x_fp = cur_x as f32;
                            let cur_y_fp = cur_y as f32;
                            let (next_x_fp, next_y_fp): (f32, f32) = (
                                cur_x_fp + x_offset + x,
                                cur_y_fp + y_offset + y,
                            );

                            let (next_x_floor, next_y_floor) = (next_x_fp.floor(), next_y_fp.floor());
                            let (next_x, next_x_offset): (usize, f32) = if next_x_floor != cur_x_fp {
                                if next_x_floor > 0. {
                                    if next_x_floor >= UNIVERSE_SIZE as f32 {
                                        (UNIVERSE_SIZE - 1, 0.0)
                                    } else {
                                        (next_x_floor as usize, next_x_fp - next_x_floor)
                                    }
                                } else {
                                    (0, 0.0)
                                }
                            } else {
                                (cur_x, cur_x_fp - (cur_x as f32))
                            };

                            let (next_y, next_y_offset): (usize, f32) = if next_y_floor != cur_y_fp {
                                if next_y_floor > 0. {
                                    if next_y_floor >= UNIVERSE_SIZE as f32 {
                                        (UNIVERSE_SIZE - 1, 0.0)
                                    } else {
                                        (next_y_floor as usize, next_y_fp - next_y_floor)
                                    }
                                } else {
                                    (0, 0.0)
                                }
                            } else {
                                (cur_y, cur_y_fp - (cur_y as f32))
                            };

                            // TODO: Translate based on the result
                            // TODO: Look into doing the translation verification in the translate command instead of
                            // while creating the translate command.  We're doing it there anyway.
                            unimplemented!();
                        }
                    }
                }
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
