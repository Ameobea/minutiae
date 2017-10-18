//! Defines the behavior of the various types of entities in the universe

use minutiae::util::iter_visible;

use super::*;

pub fn entity_driver(
    universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {
    let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);

    match &entity.state {
        &ES::Builder => {
            unimplemented!();
        },
        &ES::Dust{ shade, velocity: Velocity { x, y, x_offset, y_offset } } => {
            // Look around us for particles and mutate our velocity according to their color.
            // The goal is to have like-colored particles attracted to each other and unlike colors repelled.
            // Eventually, once we find some kind of equilibrium, we'll crystalize into a gem.

            // Calculate a new velocity based on nearby entities
            let (x_velocity_diff, y_velocity_diff) = {
                let (mut x_velocity_sum, mut y_velocity_sum) = (0.0, 0.0);

                for (entity_x, entity_y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                    let universe_index = get_index(entity_x, entity_y, UNIVERSE_SIZE);
                    for entity_index in entities.get_entities_at(universe_index) {
                        match unsafe { entities.get(*entity_index) }.state {
                            ES::Dust { shade: their_shade, .. } => {
                                // find the difference between that particle's color and our own and normalize it into
                                // the range from (-1.0, 1.0)
                                // -|(x+1)-(y+1)| + 1.0
                                let mut normalized_diff = (-1.0 * ((shade + 1.0) - (their_shade + 1.0)).abs()) + 1.0;
                                assert!(normalized_diff >= -1.0 && normalized_diff <= 1.0);
                                let (x_dist, y_dist): (isize, isize) = (entity_x as isize - cur_x as isize, entity_y as isize - cur_y as isize);

                                if x_dist != 0 {
                                    x_velocity_sum += normalized_diff * (VELOCITY_DISTANCE_FACTOR / (x_dist as f32)) * VELOCITY_SCALE;
                                }

                                if y_dist != 0 {
                                    y_velocity_sum += normalized_diff * (VELOCITY_DISTANCE_FACTOR / (y_dist as f32)) * VELOCITY_SCALE;
                                }
                            },
                            _ => (),
                        }
                    }
                }

                (x_velocity_sum, y_velocity_sum)
            };

            // Dispatch an action to calculate a new final using the calculated vector and translate according to it
            // average the new velocity into the old velocity
            let velocity_action = EA::UpdateVelocities {
                // x: (x + x_velocity_diff) / 2.0,
                // y: (y + y_velocity_diff) / 2.0,
                x: x_velocity_diff,
                y: y_velocity_diff,
            };
            self_action_executor(SelfAction::Custom(velocity_action));
        },
        &ES::Gem(shade) => {
            // I don't think gems do much
        },
    }
}

