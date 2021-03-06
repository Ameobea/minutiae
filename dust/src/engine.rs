//! Defines the behavior of the engine that regulates the bahavior of the universe as a whole.

use minutiae::action::Action;
use num::clamp;
use uuid::Uuid;

use super::*;

pub struct DancerEngine;

fn exec_cell_action(action: &OwnedAction<CS, ES, CA, EA>, cells: &mut[Cell<CS>], entities: &mut EntityContainer<CS, ES, MES>) {
    // unimplemented!();
    println!("CELL ACTION??? {:?}", action);
}

fn calc_next_position(coord: usize, offset: f32, velocity: f32) -> (usize, f32) {
    let next_fp = (coord as f32) + offset + velocity;

    if next_fp < 0.0 {
        return (0, 0.0)
    } else if next_fp >= UNIVERSE_SIZE as f32 {
        return (UNIVERSE_SIZE - 1, 0.0)
    }

    (next_fp.trunc() as usize, next_fp.fract())
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>, entities: &mut EntityContainer<CS, ES, MES>) {
    match action.action {
        Action::SelfAction(ref sa) => {
            let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);

            match *sa {
                SelfAction::Translate(x_offset, y_offset) => {
                    // make sure there are no entities where we're trying to move, and if there are set our velocity to 0
                    if entities.get_entities_at(dst_universe_index).len() > 0 {
                        let us = entities.get_verify_mut(entity_index, entity_uuid).unwrap().0;
                        match us.state {
                            ES::Dust { ref mut velocity, .. } => {
                                velocity.x = 0.0;
                                velocity.y = 0.0;
                            },
                            _ => unreachable!(),
                        }

                        return;
                    }

                    translate_entity(x_offset, y_offset, entities, entity_index, entity_uuid, UNIVERSE_SIZE);
                },
                SelfAction::Custom(ref ea) => {
                    match ea {
                        &EA::UpdateVelocities { x: new_velocity_x, y: new_velocity_y } => {
                            let (x_offset, y_offset) = {
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
                                let (next_x, next_x_offset) = calc_next_position(cur_x, x_offset, x);
                                velocity.x_offset = next_x_offset;
                                let (next_y, next_y_offset) = calc_next_position(cur_y, y_offset, y);
                                velocity.y_offset = next_y_offset;

                                (next_x as isize - cur_x as isize, next_y as isize - cur_y as isize)
                            };

                            translate(x_offset, y_offset, entities, entity_index, entity_uuid);
                        },
                        _ => unreachable!(),
                    }
                },
                _ => {
                    println!("OTHER SELF ACTION TYPE UNHANDLED");
                    unimplemented!()
                },
            }
        },
        _ => unreachable!(),
    }
}

fn exec_entity_action(action: &OwnedAction<CS, ES, CA, EA>, entities: &mut EntityContainer<CS, ES, MES>) {
    match action.action {
        Action::EntityAction{ ref action, target_entity_index, target_uuid } => {
            match action {
                &EA::InvertShade => {
                    let (entity, _) = match entities.get_verify_mut(target_entity_index, target_uuid) {
                        Some(entity) => entity,
                        None => {
                            return; // Entity has been deleted or moved.
                        },
                    };

                    match entity.state {
                        ES::Dust { ref mut shade, .. } => *shade *= -1.0, // invert the shade,
                        _ => (),
                    }
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}

pub fn exec_actions(
    universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
    self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
) {
    for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe.cells, &mut universe.entities); }
    for self_action in self_actions { exec_self_action(self_action, &mut universe.entities); }
    for entity_action in entity_actions { exec_entity_action(entity_action, &mut universe.entities); }
}

pub fn exec_actions(
    universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
    self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
) {
    for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe.cells, &mut universe.entities); }
    for self_action in self_actions { exec_self_action(self_action, &mut universe.entities); }
    for entity_action in entity_actions { exec_entity_action(entity_action); }
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
        exec_actions(universe, cell_actions, self_actions, entity_actions);
    }
}
