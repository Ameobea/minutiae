//! Defines the behavior of the various types of entities in the universe

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
            let (x_velocity, y_velocity) = unimplemented!();

            // Dispatch an action to calculate a new final using the calculated vector and translate according to it
            let velocity_action = EA::UpdateVelocities { x: x_velocity, y: y_velocity };
            self_action_executor(SelfAction::Custom(velocity_action));
        },
        &ES::Gem(shade) => {
            // I don't think gems do much
        },
    }
}
