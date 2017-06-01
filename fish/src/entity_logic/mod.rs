//! This module holds code that determines the behavior of entities.  It matches the state of each entity
//! to determine what kind it is (fish, predator, etc.) and looks at the cells ane entities around it to
//! determine what actions to take.

use super::*;

pub fn fish_driver(
    source_universe_index: usize,
    entity: &Entity<OurCellState, OurEntityState, OurMutEntityState>,
    entities: &EntityContainer<OurCellState, OurEntityState, OurMutEntityState>,
    cells: &[Cell<OurCellState>],
    cell_action_executor: &mut FnMut(OurCellAction, usize),
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>)
) {
    // fish take only one action each tick.  Their priorities are these
    //  1. Escape predators that are within their vision
    //  2. Eat any food that is adjascent to them
    //  3. Move towards any food that is within their vision but not adjascent
    //  4. Move towards nearby fish if if they are more than `SCHOOL_SPACING` units away
    //  5. Move away from nearby fish that are less than `SCHOOL_SPACING` units away

    let (cur_x, cur_y) = get_coords(source_universe_index, UNIVERSE_SIZE);
    let mut closest_predator: Option<(usize, usize, usize)> = None;
    // iterate through all visible cells and look for the predator + food item
    // which is closest to us and run away from it
    for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
        let universe_index = get_index(x, y, UNIVERSE_SIZE);
        for entity_index in entities.get_entities_at(universe_index) {
            if let OurEntityState::Predator{..} = *unsafe { &entities.get(*entity_index).state } {
                // if we found a nearby predator, calculate the distance between it and us
                // if it's less than the current minimum distance, run from this one first
                let cur_distance = manhattan_distance(cur_x, cur_y, x, y);
                match closest_predator {
                    Some((_, _, min_distance)) => {
                        if min_distance > cur_distance {
                            closest_predator = Some((x, y, cur_distance));
                        }
                    },
                    None => closest_predator = Some((x, y, cur_distance)),
                }
            }
        }
    }

    // if there's a predator to flee from, attempt to move in the opposite direction and return
    if let Some((pred_x, pred_y, _)) = closest_predator {
        // positive if predator is to the right, negative if predator is to the left
        let pred_x_offset = pred_x as isize - cur_x as isize;
        let our_x_offset = if pred_x_offset > 0 { -1 } else if pred_x_offset == 0 { 0 } else { 1 };
        let pred_y_offset = pred_y as isize - cur_y as isize;
        let our_y_offset = if pred_y_offset > 0 { -1 } else if pred_y_offset == 0 { 0 } else { 1 };
        let self_action = SelfAction::Translate(our_x_offset, our_y_offset);

        return self_action_executor(self_action);
    }

    // if there are no predators to flee from, look for the nearest food item
    let mut closest_food: Option<(usize, usize)> = None;
    for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
        let cell_index = get_index(x, y, UNIVERSE_SIZE);
        if let OurCellState::Food = cells[cell_index].state {
            // if we found a nearby food item, calculate the distance between it and us
            // if it's less than the current minimum distance, run towards this one first
            let cur_distance = manhattan_distance(cur_x, cur_y, x, y);
            match closest_food {
                Some((_, min_distance)) => {
                    if min_distance > cur_distance {
                        closest_food = Some((cell_index, cur_distance));
                    }
                },
                None => closest_food = Some((cell_index, cur_distance)),
            }
        }
    }

    if let Some((cell_index, food_distance)) = closest_food {
        // check if the food is within range of eating and, if it is, attempt to eat it.
        // if not, attempt to move towards it

        if food_distance <= 1 {
            let cell_action = OurCellAction::Eat;
            return cell_action_executor(cell_action, cell_index);
        } else {
            let (cell_x, cell_y) = get_coords(cell_index, UNIVERSE_SIZE);
            let our_x_offset = if cur_x > cell_x { -1 } else if cur_x == cell_x { 0 } else { 1 };
            let our_y_offset = if cur_y > cell_y { -1 } else if cur_y == cell_y { 0 } else { 1 };
            let self_action = SelfAction::Translate(our_x_offset, our_y_offset);
            return self_action_executor(self_action);
        }
    }

    // TODO: Implement more intelligent schooling behavior
    // if we're on the same index as another fish and aren't chasing food or running from a predator
    // pick a random direction to move and return.
    // if entities.get_entities_at(source_universe_index).len() > 1 {
        let mut mut_state_inner = entity.mut_state.take();
        let (x_offset, y_offset) = {
            let mut rng = mut_state_inner.rng.as_mut().unwrap();
            (rng.gen_range(-1, 2), rng.gen_range(-1, 2))
        };
        entity.mut_state.set(mut_state_inner);

        let self_action = SelfAction::Translate(x_offset, y_offset);
        return self_action_executor(self_action);
    // }
}

fn predator_driver(
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
