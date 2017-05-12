//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(alloc_system, conservative_impl_trait, test)]

extern crate alloc_system;
extern crate rand;
extern crate pcg;
extern crate test;
extern crate uuid;
extern crate minutae;

use std::cell::Cell as RustCell;
use std::fmt::{self, Display, Formatter};
use std::collections::HashMap;

use pcg::PcgRng;
use rand::Rng;
use uuid::Uuid;

use minutae::universe::{Universe, UniverseConf, EntityContainer};
use minutae::cell::{Cell, CellState};
use minutae::entity::{Entity, EntityState, MutEntityState};
use minutae::action::{Action, CellAction, EntityAction, OwnedAction, SelfAction};
use minutae::engine::Engine;
use minutae::engine::serial::SerialEngine;
use minutae::engine::iterator::{SerialGridIterator, SerialEntityIterator};
use minutae::generator::Generator;
use minutae::util::{
    calc_offset, get_coords, get_index, iter_visible, manhattan_distance,
    locate_entity, locate_entity_simple, EntityLocation
};
use minutae::driver::{Driver, BasicDriver};
use minutae::driver::middleware::{Middleware, UniverseDisplayer, Delay};

const TICK_DELAY_MS: u64 = 1000;
const UNIVERSE_SIZE: usize = 38;
const FISH_COUNT: usize = 150;
const PREDATOR_COUNT: usize = 2;
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
        &Action::CellAction{action: _, universe_index} => {
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
    match &action.action {
        &Action::SelfAction(ref self_action) => {
            let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
            match self_action {
                &SelfAction::Translate(x_offset, y_offset) => {
                    // this function will return `None` if the entity has been deleted
                    match universe.entities.get_verify(entity_index, entity_uuid) {
                        Some((entity, universe_index)) => {
                            // if this is the entity that we're looking for, check to see if the requested move is in bounds
                            let (cur_x, cur_y) = get_coords(universe_index, UNIVERSE_SIZE);
                            let new_x = cur_x as isize + x_offset;
                            let new_y = cur_y as isize + y_offset;
                            let dst_universe_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);

                            // verify that the supplied desination coordinates are in bounds
                            // TODO: verify that the supplied destination coordinates are within ruled bounds of destination
                            if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                                universe.entities.move_entity(entity_index, dst_universe_index);
                            }
                        },
                        None => (), // entity has been deleted, so do nothing.
                    }
                },
                &SelfAction::Custom(OurEntityAction::SetVector(x, y)) => {
                    // locate the entity that dispatched this request and mutate its state with the supplied value
                    // our implementation asserts that the entity will not have moved before this takes place, so
                    // a simple search is sufficient to locate it.
                    let (entity_index, entity_uuid) = (action.source_entity_index, action.source_uuid);
                    match universe.entities.get_verify(entity_index, entity_uuid) {
                        Some((entity, universe_index)) => {
                            match entity.state {
                                OurEntityState::Predator{food: _, ref mut direction} => {
                                    *direction = Some((x, y));
                                },
                                _ => unreachable!(),
                            }
                        },
                        None => (), // entity has been deleted; do nothing.
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
    match &action.action {
        &Action::EntityAction{action: ref entity_action, target_entity_index, target_uuid} => {
            match entity_action {
                &OurEntityAction::EatFish => {
                    // check to see if the shark (source entity) is still alive
                    let (source_entity_index, source_entity_uuid) = (action.source_entity_index, action.source_uuid);
                    match universe.entities.get_verify(source_entity_index, source_entity_uuid) {
                        Some((source_entity, source_universe_index)) => {
                            // check to see if the fish is still alive and that we're in range to eat it
                            match universe.entities.get_verify(target_entity_index, target_uuid) {
                                Some((target_entity, target_universe_index)) => {
                                    let (src_x, src_y) = get_coords(source_universe_index, UNIVERSE_SIZE);
                                    let (entity_x, entity_y) = get_coords(target_universe_index, UNIVERSE_SIZE);
                                    if manhattan_distance(src_x, src_y, entity_x, entity_y) > 1 {
                                        return;
                                    }

                                    // eat the fish, removing its entity + its entry in the meta `HashMap` and incrementing our food count.
                                    let eaten_fish = universe.entities.remove(target_entity_index);
                                    debug_assert_eq!(eaten_fish.uuid, target_uuid);

                                    // increment the food value of the source entity
                                    match source_entity.state {
                                        OurEntityState::Predator{ref mut food, direction: _} => { *food += 1 },
                                        _ => unreachable!(),
                                    }
                                },
                                None => (),
                            }
                        },
                        None => (),
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
    ) {
        println!("Generating world...");
        // let mut rng = PcgRng::new_unseeded().with_stream(self.0);
        let length = conf.size * conf.size;
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            // populate the world with water cells
            cells.push(Cell{state: OurCellState::Water});
        }

        let mut entities = vec![Vec::new(); length];
        let mut rng = PcgRng::new_unseeded();
        rng.set_stream(10101010101);

        // populate the world with `FISH_COUNT` randomly placed fish
        let origin_entity = Entity::new(OurEntityState::Fish{food: 0}, OurMutEntityState {rng: Some(rng.clone())});
        for _ in 0..FISH_COUNT {
            let index = rng.gen_range(0, UNIVERSE_SIZE * UNIVERSE_SIZE);
            let entity = origin_entity.clone();
            entities[index].push(entity);
        }

        // populate the world with `PREDATOR_COUNT` random placed predators
        let origin_predator = Entity::new(
            OurEntityState::Predator{food: 0, direction: None},
            OurMutEntityState {rng: Some(rng.clone())}
        );
        for _ in 0..PREDATOR_COUNT {
            let index = rng.gen_range(0, UNIVERSE_SIZE * UNIVERSE_SIZE);
            let entity = origin_predator.clone();
            entities[index].push(entity);
        }

        (cells, entities)
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
    source_universe_index: usize,
    entity: &Entity<OurCellState, OurEntityState, OurMutEntityState>,
    entities: &EntityContainer<OurCellState, OurEntityState, OurMutEntityState>,
    cells: &[Cell<OurCellState>],
    cell_action_executor: &mut FnMut(OurCellAction, usize),
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>),
    entity_action_executor: &mut FnMut(OurEntityAction, usize, Uuid)
) {
    let (cur_x, cur_y) = get_coords(source_universe_index, UNIVERSE_SIZE);
    match entity.state {
        OurEntityState::Fish{food: _} => {
            // fish take only one action each tick.  Their priorities are these
            //  1. Escape predators that are within their vision
            //  2. Eat any food that is adjascent to them
            //  3. Move towards any food that is within their vision but not adjascent
            //  4. Move towards nearby fish if if they are more than `SCHOOL_SPACING` units away
            //  5. Move away from nearby fish that are less than `SCHOOL_SPACING` units away

            let mut closest_predator: Option<(usize, usize, usize)> = None;
            // iterate through all visible cells and look for the predator + food item which is closest to us and run away from it
            for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                let universe_index = get_index(x, y, UNIVERSE_SIZE);
                for entity_index in entities.get_entities_at(universe_index) {
                    match entities.get(*entity_index).state {
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
            let mut closest_food: Option<(usize, usize)> = None;
            for (x, y) in iter_visible(cur_x, cur_y, VIEW_DISTANCE, UNIVERSE_SIZE) {
                let cell_index = get_index(x, y, UNIVERSE_SIZE);
                match cells[cell_index].state {
                    OurCellState::Food => {
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
                    },
                    _ => (),
                }
            }

            match closest_food {
                Some((cell_index, food_distance)) => {
                    // check if the food is within range of eating and, if it is, attempt to eat it.
                    // if not, attempt to move towards it

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
        OurEntityState::Predator{food: _, direction} => {
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
                        let fish_index = get_index(fish_x, fish_y, UNIVERSE_SIZE);
                        return entity_action_executor(OurEntityAction::EatFish, fish_index, fish_uuid);
                    } else {
                        let self_action = SelfAction::Translate(x_offset, y_offset);
                        return self_action_executor(self_action);
                    }
                },
                None => (),
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
                            // movement would cause us to try to leave the universe, so generate a new random vector
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
    use minutae::universe;

    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let engine: OurEngineType = Box::new(OurEngine {});

    let universe = universe::Universe::new(
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
    use minutae::universe;

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

    let mut middleware = FoodSpawnerMiddleware(PcgRng::new_unseeded());

    b.iter(|| {
        middleware.before_render(&mut universe);
        engine.step(&mut universe)
    });

    println!("{:?}", universe);
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

#[bench]
fn uuid_pcg(b: &mut test::Bencher) {
    use pcg::PcgRng;
    use rand::Rng;

    let mut rng = PcgRng::new_unseeded();
    rng.set_stream(9182837465);
    let mut buf = vec![0u8; 16];

    b.iter(|| {
        rng.fill_bytes(&mut buf);
        Uuid::from_bytes(&buf)
    })
}