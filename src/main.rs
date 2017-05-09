//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(conservative_impl_trait, test)]

extern crate itertools;
extern crate rand;
extern crate pcg;
extern crate test;
extern crate uuid;

use std::cell::Cell as RustCell;
use std::fmt::{self, Display, Formatter};
use std::collections::HashMap;

use pcg::PcgRng;
use rand::Rng;
use uuid::Uuid;

mod universe;
mod cell;
mod entity;
mod action;
mod engine;
mod generator;
mod util;
mod driver;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, CellAction, EntityAction, OwnedAction, SelfAction};
use engine::Engine;
use engine::serial::SerialEngine;
use engine::iterator::{SerialGridIterator, SerialEntityIterator};
use generator::Generator;
use util::{calc_offset, get_coords, get_index, iter_visible, manhattan_distance};
use driver::{Driver, BasicDriver};
use driver::middleware::{Middleware, UniverseDisplayer, Delay};

const TICK_DELAY_MS: u64 = 12;
const UNIVERSE_SIZE: usize = 38;
const ENTITY_COUNT: usize = 50;
const VIEW_DISTANCE: usize = 4;
const SCHOOL_SPACING: usize = 2;
// there's a one in `this` chance of spawning a food cluster each tick
const FOOD_SPAWN_RARITY: usize = 4;
// this number of food cells are spawned (minus overlaps)
const FOOD_SPAWN_COUNT: usize = 9;
const FOOD_SPAWN_RADIUS: isize = 7;

#[derive(Clone, Debug)]
enum OurCellState {
    Water,
    Food,
}

impl CellState for OurCellState {}

impl Display for OurCellState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let val = match self {
            &OurCellState::Water => ' ',
            &OurCellState::Food => '\'',
        };

        write!(formatter, "{}", val)
    }
}

#[derive(Clone, Debug)]
enum OurEntityState {
    Fish {
        food: usize,
    },
    Predator {
        food: usize,
        direction: Option<(i8, i8)>,
    },
}

impl EntityState<OurCellState> for OurEntityState {}

impl Display for OurEntityState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let val = match self {
            &OurEntityState::Predator{food: _, direction: _} => 'V',
            &OurEntityState::Fish{food: _} => '^',
        };
        write!(formatter, "{}", val)
    }
}

struct OurMutEntityState {
    rng: Option<PcgRng>,
}

impl Clone for OurMutEntityState {
    fn clone(&self) -> Self {
        let mut true_rng = rand::thread_rng();
        let mut prng = PcgRng::new_unseeded();
        prng.set_stream(true_rng.next_u64());

        OurMutEntityState {
            rng: Some(prng)
        }
    }
}

impl MutEntityState for OurMutEntityState {}

impl Default for OurMutEntityState {
    fn default() -> OurMutEntityState {
        OurMutEntityState {
            rng: None,
        }
    }
}

#[derive(Debug)]
enum OurCellAction {
    Eat, // The only thing that we can really do to the world right now is eat food
}

impl CellAction<OurCellState> for OurCellAction {}

#[derive(Debug)]
enum OurEntityAction {
    EatFish,
    MakeBaby,
    SetVector(i8, i8),
}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

type OurEngineType = Box<
        SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction,
        OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>
    >;

struct OurEngine {}

fn exec_cell_action(
    action: &OwnedAction<OurCellState, OurEntityState, OurCellAction, OurEntityAction>,
    universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>
) {
    match &action.action {
        &Action::CellAction{action: ref cell_action, x_offset, y_offset} => {
            let (cur_universe_index, _) = (action.source_universe_index, action.source_entity_index);
            let (cur_x, cur_y) = get_coords(cur_universe_index, UNIVERSE_SIZE);
            let cell_x = cur_x as isize + x_offset;
            let cell_y = cur_y as isize + y_offset;
            if cell_x >= 0 && cell_x < UNIVERSE_SIZE as isize && cell_y >= 0 && cell_y < UNIVERSE_SIZE as isize &&
                x_offset.abs() as usize <= VIEW_DISTANCE && x_offset.abs() as usize <= VIEW_DISTANCE
            {
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
    match &action.action {
        &Action::SelfAction(ref self_action) => {
            match self_action {
                &SelfAction::Translate(x_offset, y_offset) => {
                    let (cur_universe_index, cur_entity_index) = (action.source_universe_index, action.source_entity_index);
                    let (cur_x, cur_y) = get_coords(cur_universe_index, UNIVERSE_SIZE);
                    let new_x = cur_x as isize + x_offset;
                    let new_y = cur_y as isize + y_offset;

                    // verify that the supplied desination coordinates are in bounds
                    // TODO: verify that the supplied destination coordinates are within ruled bounds of destination
                    if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                        // check to make sure that the entity we're searching for is in its expected location
                        // println!("{:?}", universe.entities[cur_universe_index]);
                        let new_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);
                        let entity = if universe.entities[cur_universe_index].len() <= cur_entity_index ||
                            universe.entities[cur_universe_index][cur_entity_index].uuid != action.source_uuid
                        {
                            let real_entity_index = universe.entities[cur_universe_index]
                                .iter()
                                .position(|& ref entity| entity.uuid == action.source_uuid)
                                .expect("The requested entity is not found at any index at the specified universe index!");
                            universe.entities[cur_universe_index].remove(real_entity_index)
                        } else {
                            universe.entities[cur_universe_index].remove(cur_entity_index)
                        };
                        universe.entity_meta.insert(entity.uuid, (new_x as usize, new_y as usize))
                            .expect("No entry found in entity meta HashMap for pre-existing Entity!");
                        universe.entities[new_index].push(entity);
                    }
                }
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
    match &action.action {
        &Action::EntityAction{action: ref entity_action, x_offset, y_offset, target_uuid} => {
            match entity_action {
                &OurEntityAction::EatFish => {
                    // check to see if the fish is still where it's expected to be
                    let (src_x, src_y) = get_coords(action.source_entity_index, UNIVERSE_SIZE);
                    let (expected_x, expected_y) = (src_x as isize + x_offset, src_y as isize + y_offset);
                    let mut universe_index = get_index(expected_x as usize, expected_y as usize, UNIVERSE_SIZE);

                    let entity_index_res = universe.entities[universe_index]
                        .iter()
                        .position(|& ref entity| entity.uuid == target_uuid);
                    let (entity_x, entity_y, entity_index, moved) = match entity_index_res {
                        Some(entity_index) => (expected_x as usize, expected_y as usize, entity_index, false),
                        None => {
                            // The entity must have moved or been deleted, so we have to consult the meta `HashHap`.
                            match universe.entity_meta.get(&target_uuid) {
                                Some(&(real_x, real_y)) => {
                                    let real_universe_index = get_index(real_x, real_y, UNIVERSE_SIZE);
                                    let real_entity_index = universe.entities[real_universe_index]
                                        .iter()
                                        .position(|& ref entity| entity.uuid == target_uuid)
                                        .expect("Requested entity not found at position pointed to in meta `HashMap`!");

                                    (real_x, real_y, real_entity_index, true)
                                },
                                None => {
                                    // the entity must have been deleted, so nothing to do.
                                    return;
                                },
                            }
                        }
                    };

                    // if the fish moved, we need to check if it's still in range and if not, abort
                    if moved {
                        if manhattan_distance(src_x, src_y, entity_x, entity_y) > 1 {
                            return;
                        }

                        // recalculate universe index
                        universe_index = get_index(entity_x, entity_y, UNIVERSE_SIZE);
                    }

                    // eat the fish, removing its entity and incrementing our food count.
                    let eaten_fish = universe.entities[universe_index].remove(entity_index);
                    debug_assert_eq!(eaten_fish.uuid, target_uuid);

                    let source_entity_index = if
                        universe.entities[action.source_universe_index].len() > action.source_entity_index &&
                        universe.entities[action.source_universe_index][action.source_entity_index].uuid == action.source_uuid
                    {
                        action.source_entity_index
                    } else {
                        // not possible for the source entity to have moved due to our entity's behaviour
                        universe.entities[action.source_universe_index]
                            .iter()
                            .position(|& ref entity| entity.uuid == target_uuid)
                            .expect("Source entity not found at position pointed to in meta `HashMap`!")
                    };
                    match universe.entities[action.source_universe_index][source_entity_index].state {
                        // TODO: Deal with the apparent copy happening here and actually increment food
                        OurEntityState::Predator{mut food, direction: _} => food += 1,
                        _ => unreachable!(),
                    }
                },
                &OurEntityAction::MakeBaby => unimplemented!(),
                &OurEntityAction::SetVector(_, _) => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}

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
        // process actions in order of cell actions, then self actions, and finally entity actions
        for cell_action in cell_actions {
            exec_cell_action(cell_action, universe);
        }

        // println!("{:?}", self_actions);
        for self_action in self_actions {
            exec_self_action(self_action, universe);
        }

        for entity_action in entity_actions {
            exec_entity_action(entity_action, universe);
        }
    }
}

struct OurWorldGenerator(u64);

impl Generator<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction> for OurWorldGenerator {
    fn gen(
        &mut self, conf: &UniverseConf
    ) -> (
        Vec<Cell<OurCellState>>,
        Vec<Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>>,
        HashMap<Uuid, (usize, usize)>
    ) {
        println!("Generating world...");
        // let mut rng = PcgRng::new_unseeded().with_stream(self.0);
        let length = conf.size * conf.size;
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            // let baby_cell = Cell{state: match rng.gen() {
            //     false => OurCellState::Empty,
            //     true => OurCellState::Filled,
            // }};
            // cells.push(baby_cell);
            cells.push(Cell{state: OurCellState::Water});
        }
        let mut entities = vec![Vec::new(); length];

        let mut entity_meta = HashMap::new();
        let mut rng = PcgRng::new_unseeded();
        rng.set_stream(10101010101);
        let origin_entity = Entity::new(
            OurEntityState::Fish{food: 0},
            OurMutEntityState {rng: Some(rng.clone())}
        );

        for _ in 0..ENTITY_COUNT {
            let index = rng.gen_range(0, UNIVERSE_SIZE * UNIVERSE_SIZE);
            let entity = origin_entity.clone();
            entity_meta.insert(entity.uuid, get_coords(index, UNIVERSE_SIZE));
            entities[index].push(entity);
        }

        (cells, entities, entity_meta)
    }
}

fn our_cell_mutator<'a>(_: usize, _: &[Cell<OurCellState>]) -> Option<OurCellState> {
    // Some(match cells[index].state {
    //     OurCellState::Empty => OurCellState::Filled,
    //     OurCellState::Filled => OurCellState::Empty,
    // })
    None
}

/// This function determines the core logic of the simulation.  Every entity evaluates this function every tick of the
/// simulation.  Actions are sent to the various executors and dispatched in batch after all entities have submitted them.
fn our_entity_driver<'a>(
    cur_x: usize,
    cur_y: usize,
    cur_state: &OurEntityState,
    mut_state: &RustCell<OurMutEntityState>,
    entities: &[Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>],
    cells: &[Cell<OurCellState>],
    cell_action_executor: &mut FnMut(OurCellAction, isize, isize),
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>),
    entity_action_executor: &mut FnMut(OurEntityAction, isize, isize, Uuid)
) {
    match cur_state {
        &OurEntityState::Fish{food: _} => {
                // fish take only one action each tick.  Their priorities are these
            //  1. Escape predators that are within their vision
            //  2. Eat any food that is adjascent to them
            //  3. Move towards any food that is within their vision but not adjascent
            //  4. Move towards nearby fish if if they are more than `SCHOOL_SPACING` units away
            //  5. Move away from nearby fish that are less than `SCHOOL_SPACING` units away

            let mut closest_predator: Option<(usize, usize, usize)> = None;
            // iterate through all visible cells and look for the predator + food item which is closest to us and run away from it
            for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                let index = get_index(x, y, UNIVERSE_SIZE);
                for entity in &entities[index] {
                    match entity.state {
                        OurEntityState::Predator{food: _, direction: _} => {
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
                        },
                        _ => (),
                    }
                }
            }

            // if there's a predator to flee from, attempt to move in the opposite direction and return
            match closest_predator {
                Some((pred_x, pred_y, _)) => {
                    // positive if predator is to the right, negative if predator is to the left
                    let pred_x_offset = pred_x as isize - cur_x as isize;
                    let our_x_offset = if pred_x_offset > 0 { -1 } else if pred_x_offset == 0 { 0 } else { 1 };
                    let pred_y_offset = pred_y as isize - cur_y as isize;
                    let our_y_offset = if pred_y_offset > 0 { -1 } else if pred_y_offset == 0 { 0 } else { 1 };
                    let self_action = SelfAction::Translate(our_x_offset, our_y_offset);

                    return self_action_executor(self_action);
                },
                None => (),
            }

            // if there are no predators to flee from, look for the nearest food item
            let mut closest_food: Option<(usize, usize, usize)> = None;
            for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                let index = get_index(x, y, UNIVERSE_SIZE);
                match cells[index].state {
                    OurCellState::Food => {
                        // if we found a nearby food item, calculate the distance between it and us
                        // if it's less than the current minimum distance, run towards this one first
                        let cur_distance = manhattan_distance(cur_x, cur_y, x, y);
                        match closest_food {
                            Some((_, _, min_distance)) => {
                                if min_distance > cur_distance {
                                    closest_food = Some((x, y, cur_distance));
                                }
                            },
                            None => closest_food = Some((x, y, cur_distance)),
                        }
                    },
                    _ => (),
                }
            }

            match closest_food {
                Some((food_x, food_y, food_distance)) => {
                    // check if the food is within range of eating and, if it is, attempt to eat it.
                    // if not, attempt to move towards it
                    let x_offset = food_x as isize - cur_x as isize;
                    let y_offset = food_y as isize - cur_y as isize;

                    if food_distance <= 1 {
                        let cell_action = OurCellAction::Eat;
                        return cell_action_executor(cell_action, x_offset, y_offset);
                    } else {
                        let our_x_offset = if x_offset < 0 { -1 } else if x_offset == 0 { 0 } else { 1 };
                        let our_y_offset = if y_offset < 0 { -1 } else if y_offset == 0 { 0 } else { 1 };
                        let self_action = SelfAction::Translate(our_x_offset, our_y_offset);
                        return self_action_executor(self_action);
                    }
                },
                None => (),
            }

            // TODO: Implement more intelligent schooling behavior
            // if we're on the same index as another fish and aren't chasing food or running from a predator, pick a random
            // direction to move and return.
            if entities[get_index(cur_x, cur_y, UNIVERSE_SIZE)].len() > 1 {
                let mut mut_state_inner = mut_state.take();
                let (x_offset, y_offset) = {
                    let mut rng = mut_state_inner.rng.as_mut().unwrap();
                    (rng.gen_range(-1, 2), rng.gen_range(-1, 2))
                };
                mut_state.set(mut_state_inner);

                let self_action = SelfAction::Translate(x_offset, y_offset);
                return self_action_executor(self_action);
            }
        },
        &OurEntityState::Predator{food: _, direction} => {
            // 1. If we're adjascent to a fish, eat it.
            // 2. If we see a fish, move towards it.
            // 3. If we don't see any fish, pick a random vector (if we don't already have one picked) and move that way.

            // if there are no predators to flee from, look for the nearest food item
            let mut closest_fish: Option<(usize, usize, Uuid, usize)> = None;
            for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                let index = get_index(x, y, UNIVERSE_SIZE);
                for entity in &entities[index] {
                    match entity.state {
                        OurEntityState::Fish{food: _} => {
                            // if we found a nearby fish, calculate the distance between it and us
                            // if it's less than the current minimum distance, run towards this one first
                            let cur_distance = manhattan_distance(cur_x, cur_y, x, y);
                            match closest_fish {
                                Some((_, _, _, min_distance)) => {
                                    if min_distance > cur_distance {
                                        closest_fish = Some((x, y, entity.uuid, cur_distance));
                                    }
                                },
                                None => closest_fish = Some((x, y, entity.uuid, cur_distance)),
                            }
                        },
                        _ => (),
                    }
                }
            }

            // If we can see a fish if we're adjascent to it, eat it.  If not, move towards it.
            match closest_fish {
                Some((fish_x, fish_y, fish_uuid, fish_distance)) => {
                    let (x_offset, y_offset) = calc_offset(cur_x, cur_y, fish_x, fish_y);
                    if fish_distance <= 1 {
                        return entity_action_executor(OurEntityAction::EatFish, x_offset, y_offset, fish_uuid);
                    } else {
                        let our_x_offset = if x_offset > 0 { -1 } else if x_offset == 0 { 0 } else { 1 };
                        let our_y_offset = if y_offset > 0 { -1 } else if y_offset == 0 { 0 } else { 1 };
                        let self_action = SelfAction::Translate(our_x_offset, our_y_offset);
                        return self_action_executor(self_action);
                    }
                },
                None => (),
            }

            // we can't see any fish, so pick a random direction to swim in (if we haven't already picked one) and swim that way
            let (x_dir, y_dir) = match direction {
                Some(vector) => vector,
                None => {
                    let mut mut_state_inner = mut_state.take();
                    let (x_dir, y_dir): (i8, i8) = {
                        let mut rng = mut_state_inner.rng.as_mut().unwrap();
                        let mut vector: (i8, i8) = (0, 0);

                        while vector == (0, 0) {
                            vector = (rng.gen_range(-1, 2), rng.gen_range(-1, 2));
                        }

                        vector
                    };
                    let self_action = SelfAction::Custom(OurEntityAction::SetVector(x_dir, y_dir));
                    self_action_executor(self_action);

                    (x_dir, y_dir)
                }
            };

            let self_action = SelfAction::Translate(x_dir as isize, y_dir as isize);
            self_action_executor(self_action);
        }
    }
}

/// Add step onto the end of each simulation cycle that has a chance of spawning some food into the world for the fish to eat
struct FoodSpawnerMiddleware(PcgRng);

impl Middleware<
    OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, OurEngineType
> for FoodSpawnerMiddleware {
    fn before_render(
        &mut self, universe: &mut Universe<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction>
    ) {
        let mut rng = &mut self.0;
        if rng.gen_range(0, FOOD_SPAWN_RARITY) == 0 {
            let food_spawn_x = rng.gen_range(0, UNIVERSE_SIZE);
            let food_spawn_y = rng.gen_range(0, UNIVERSE_SIZE);
            let mut spawned_food = 0;
            while spawned_food < FOOD_SPAWN_COUNT {
                // attempt to place a food item at the calculated offset
                let spawn_x_offset = rng.gen_range(-FOOD_SPAWN_RADIUS, FOOD_SPAWN_RADIUS);
                let target_x = food_spawn_x as isize + spawn_x_offset as isize;
                let spawn_y_offset = rng.gen_range(-FOOD_SPAWN_RADIUS, FOOD_SPAWN_RADIUS);
                let target_y = food_spawn_y as isize + spawn_y_offset as isize;
                
                if target_x >= 0 && target_x < UNIVERSE_SIZE as isize && target_y >= 0 && target_y < UNIVERSE_SIZE as isize {
                    let target_index = get_index(target_x as usize, target_y as usize, UNIVERSE_SIZE);
                    universe.cells[target_index].state = OurCellState::Food;
                    spawned_food += 1;
                }
            }
        }
    }
}

impl FoodSpawnerMiddleware {
    pub fn new() -> Self {
        let mut true_rng = rand::thread_rng();
        let mut prng = PcgRng::new_unseeded();
        prng.set_stream(true_rng.next_u64());

        FoodSpawnerMiddleware(prng)
    }
}

fn main() {
    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let engine: OurEngineType = Box::new(OurEngine {});

    let universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19093929992071),
        our_cell_mutator,
        our_entity_driver,
    );

    let driver = BasicDriver::new();
    driver.init(universe, engine, &mut [
        Box::new(UniverseDisplayer {}),
        Box::new(Delay(TICK_DELAY_MS)),
        Box::new(FoodSpawnerMiddleware::new()),
    ]);
}

#[bench]
fn universe_step(b: &mut test::Bencher) {
    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let mut engine: Box<
        SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction,
        OurEntityAction,SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>
    > = Box::new(OurEngine {});

    let mut universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19200064321271),
        our_cell_mutator,
        our_entity_driver,
    );

    b.iter(|| engine.step(&mut universe))
}

#[bench]
fn hashset_remove_insert(b: &mut test::Bencher) {
    let mut hs = ::std::collections::HashSet::new();
    for i in 0..10000 {
        hs.insert(i);
    }

    b.iter(|| {
        hs.remove(&9);
        for i in 10..5000 {
            hs.remove(&i);
            hs.insert(i - 1);
        }
    })
}
