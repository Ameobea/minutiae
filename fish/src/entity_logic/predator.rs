//! Defines the behavior of predator entities.  Predators swim around in a straight line until they see fish.  They then
//! chase the fish and eat them if they catch them.

use super::*;

pub fn predator_driver(
    direction: Option<(i8, i8)>,
    source_universe_index: usize,
    entity: &Entity<OurCellState, OurEntityState, OurMutEntityState>,
    entities: &EntityContainer<OurCellState, OurEntityState, OurMutEntityState>,
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>),
    entity_action_executor: &mut FnMut(OurEntityAction, usize, Uuid)
) {
    // 1. If we're adjascent to a fish, eat it.
    // 2. If we see a fish, move towards it.
    // 3. If we don't see any fish, pick a random vector (if we don't already have one picked) and move that way.

    // if there are no predators to flee from, look for the nearest food item
    let (cur_x, cur_y) = get_coords(source_universe_index, UNIVERSE_SIZE);
    let mut closest_fish: Option<(usize, usize, usize, Uuid, usize)> = None;
    for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
        let universe_index = get_index(x, y, UNIVERSE_SIZE);
        for entity_index in entities.get_entities_at(universe_index) {
            let target_entity = unsafe { entities.get(*entity_index) };
            if let OurEntityState::Fish{..} = target_entity.state {
                // if we found a nearby fish, calculate the distance between it and us
                // if it's less than the current minimum distance, run towards this one first
                let cur_distance = manhattan_distance(cur_x, cur_y, x, y);
                match closest_fish {
                    Some((_, _, _, _, min_distance)) => {
                        if min_distance > cur_distance {
                            closest_fish = Some((x, y, *entity_index, target_entity.uuid, cur_distance));
                        }
                    },
                    None => closest_fish = Some((x, y, *entity_index, target_entity.uuid, cur_distance)),
                }
            }
        }
    }

    // If we can see a fish if we're adjascent to it, eat it.  If not, move towards it.
    if let Some((fish_x, fish_y, fish_entity_index, fish_uuid, fish_distance)) = closest_fish {
        let (x_offset, y_offset) = calc_offset(cur_x, cur_y, fish_x, fish_y);
        if fish_distance <= 1 {
            return entity_action_executor(OurEntityAction::EatFish, fish_entity_index, fish_uuid);
        } else {
            let self_action = SelfAction::Translate(x_offset, y_offset);
            return self_action_executor(self_action);
        }
    }

    // we can't see any fish, so pick a random direction to swim in (if we haven't already picked one) and swim that way
    let (x_dir, y_dir) = {
        let mut get_random_vector = || {
            let mut mut_state_inner = entity.mut_state.take();

            let mut vector: (i8, i8) = (0, 0);

            vector = {
                let mut rng = mut_state_inner.rng.as_mut().unwrap();
                while vector == (0, 0) {
                    vector = (rng.gen_range(-1, 2), rng.gen_range(-1, 2));
                }

                vector
            };

            entity.mut_state.set(mut_state_inner);
            let self_action = SelfAction::Custom(OurEntityAction::SetVector(vector.0, vector.1));
            self_action_executor(self_action);

            vector
        };

        match direction {
            Some((x, y)) => {
                let x_dst = cur_x as isize + x as isize;
                let y_dst = cur_y as isize + y as isize;
                if x_dst < 0 || x_dst as usize >= UNIVERSE_SIZE || y_dst < 0 || y_dst as usize >= UNIVERSE_SIZE {
                    // movement would cause us to try to leave the universe,
                    // so generate a new random vector
                    get_random_vector()
                } else { (x, y) }
            },
            None => {
                get_random_vector()
            }
        }
    };

    let self_action = SelfAction::Translate(x_dir as isize, y_dir as isize);
    self_action_executor(self_action);
}
