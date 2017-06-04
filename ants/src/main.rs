//! Small ant colony simulation with pheremone trails and simulated foraging behavior.

// temp during development
#![allow(dead_code, unused_variables)]

extern crate minutiae;
extern crate pcg;
extern crate rand;
extern crate uuid;

use pcg::PcgRng;
use uuid::Uuid;
use rand::{Rng, SeedableRng};
use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialGridIterator, SerialEntityIterator};
use minutiae::driver::middleware::MinDelay;
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};

extern {
    pub fn canvas_render(pixbuf_ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 800;
const ANT_COUNT: usize = 17;
const FOOD_DEPOSIT_COUNT: usize = 25;
const FOOD_DEPOSIT_SIZE: usize = 76;
const FOOD_DEPOSIT_RADIUS: usize = 8;
const MAX_FOOD_QUANTITY: u16 = 4000;
const PRNG_SEED: [u64; 2] = [198918237842, 9];
const ANT_FOOD_CAPACITY: usize = 12;

const UNIVERSE_LENGTH: usize = UNIVERSE_SIZE * UNIVERSE_SIZE;

#[derive(Clone)]
struct Pheremones {
    searching: u16, // Indicates that an ant was on this while searching for food
    found: u16, // Indicates that an ant was walking on this square while carrying food
}

impl Pheremones {
    pub fn new() -> Self {
        Pheremones {
            searching: 0,
            found: 0,
        }
    }
}

#[derive(Clone, PartialEq)]
enum CellContents {
    Empty,
    Filled(u8),
    Food(u16),
    Anthill,
}

#[derive(Clone)]
struct CS {
    pheremones: Pheremones,
    contents: CellContents,
}

impl CellState for CS {}

#[derive(Clone)]
enum AntState {
    Wandering, // Ant is currently searching the world for food
    FollowingTrailToFood, // Ant is following a trail that it thinks leads to food
    ReturningWithFood, // Ant is carrying food and attempting to bring it back to the anthill
}

#[derive(Clone)]
enum ES {
    Ant {
        state: AntState,
        held_food: usize,
    }
}

impl EntityState<CS> for ES {}

impl ES {
    pub fn new_ant() -> Entity<CS, Self, MES> { Entity::new(ES::Ant {state: AntState::Wandering, held_food: 0}, MES::default()) }
}

#[derive(Clone, Default)]
struct MES {}

impl MutEntityState for MES {}

fn color_calculator(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    unimplemented!(); // TODO
}

enum CA {
    LaySearchPheremone, // Deposit a pheremone on the current coordinate indicating that we were here while searching for food
    LayFoundPheremone, // Deposit a pheremone on the current coordinate indicating that we're returning with food
    CollectFood(usize), // Collects some food from the specified universe index
}

impl CellAction<CS> for CA {}

enum EA {

}

impl EntityAction<CS, ES> for EA {}

struct WorldGenerator;

/// Given a coordinate, selects a point that's less than `size` units away from the source coordinate as calculated using
/// Manhattan distance.  The returned coordinate is guarenteed to be valid and within the universe.
fn rand_coord_near(rng: &mut PcgRng, src_index: usize, max_distance: usize) -> usize {
    let distance = rng.gen_range(0, max_distance + 1) as isize;
    loop {
        let x_mag = rng.gen_range(0, distance);
        let y_mag = distance - x_mag;

        let (x_offset, y_offset) = match rng.gen_range(0, 3) {
            0 => (x_mag, y_mag),
            1 => (-x_mag, y_mag),
            2 => (x_mag, -y_mag),
            3 => (-x_mag, -y_mag),
            _ => unreachable!(),
        };

        let (x, y) = get_coords(src_index, UNIVERSE_SIZE);
        let dst_x = x as isize + x_offset;
        let dst_y = y as isize + y_offset;
        if dst_x >= 0 && dst_x < UNIVERSE_SIZE as isize && dst_y >= 0 && dst_y < UNIVERSE_SIZE as isize {
            return get_index(dst_x as usize, dst_y as usize, UNIVERSE_SIZE);
        }
    }
}

impl Generator<CS, ES, MES, CA, EA> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        let mut rng = PcgRng::from_seed(PRNG_SEED);
        let mut cells = vec![Cell{state: CS {pheremones: Pheremones::new(), contents: CellContents::Empty}}; UNIVERSE_LENGTH];
        let mut entities = vec![Vec::new(); UNIVERSE_LENGTH];

        // Pick location of anthill and spawn ants around it
        let anthill_index = rng.gen_range(0, UNIVERSE_LENGTH);
        cells[anthill_index].state.contents = CellContents::Anthill;
        let (hill_x, hill_y) = get_coords(anthill_index, UNIVERSE_SIZE);
        let mut spawned_ants = 0;
        // spawn ants close to the anthill to start off
        while spawned_ants < ANT_COUNT {
            let index = rand_coord_near(&mut rng, anthill_index, 4);
            if entities[index].len() == 0 {
                entities[index].push(ES::new_ant());
                spawned_ants += 1;
            }
        }

        // Create food deposits scattered around the world
        for _ in 0..FOOD_DEPOSIT_COUNT {
            // pick a center location for the food cluster
            let center_index = rng.gen_range(0, UNIVERSE_LENGTH);
            // place at most `FOOD_DEPOSIT_SIZE` units of food in the area around the center
            for _ in 0..FOOD_DEPOSIT_SIZE {
                let food_index = rand_coord_near(&mut rng, center_index, FOOD_DEPOSIT_RADIUS);
                if cells[food_index].state.contents != CellContents::Anthill {
                    let food_quantity = rng.gen_range(1, MAX_FOOD_QUANTITY);
                    cells[food_index].state.contents = CellContents::Food(food_quantity)
                }
            }
        }

        (cells, entities)
    }
}

/// No-op cell mutator since we aren't mutating cells in this simulation
fn cell_mutator(_: usize, _: &[Cell<CS>]) -> Option<CS> { None }

fn entity_driver(
    universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {
    match entity.state {
        ES::Ant{ref state, ..} => match state {
            &AntState::Wandering => {
                // lay some pheremone on the current location to indicate that we walked here while searching for food
                cell_action_executor(CA::LaySearchPheremone, universe_index);
                // TODO: Wander
                unimplemented!(); // TODO
            },
            &AntState::FollowingTrailToFood => {
                // TODO: Follow trail to the food
                unimplemented!(); // TODO
            },
            &AntState::ReturningWithFood => {
                // lay some pheremone on the current location to indicate that we walked here while returning food to the nest
                cell_action_executor(CA::LayFoundPheremone, universe_index);
                // TODO: Follow the trail back to the nest
                unimplemented!(); // TODO
            },
        }
    }
}

struct AntEngine;

fn exec_cell_action(owned_action: &OwnedAction<CS, ES, CA, EA>, cells: &mut [Cell<CS>], entities: &mut EntityContainer<CS, ES, MES>) {
    let (mut entity, entity_universe_index) = match entities.get_verify_mut(owned_action.source_entity_index, owned_action.source_uuid) {
        Some((entity, universe_index)) => (entity, universe_index),
        None => { return; }, // The entity been deleted, so abort.
    };

    match &owned_action.action {
        &Action::CellAction {ref action, ..} => match action {
            &CA::LaySearchPheremone => {
                unsafe { cells.get_unchecked_mut(entity_universe_index).state.pheremones.searching += 1 };
            },
            &CA::LayFoundPheremone => {
                unsafe { cells.get_unchecked_mut(entity_universe_index).state.pheremones.found += 1 };
            },
            &CA::CollectFood(dst_universe_index) => {
                let (src_x, src_y) = get_coords(entity_universe_index, UNIVERSE_SIZE);
                let (dst_x, dst_y) = get_coords(dst_universe_index, UNIVERSE_SIZE);
                if manhattan_distance(src_x, src_y, dst_x, dst_y) <= 1 {
                    let mut cell_state = unsafe { &mut cells.get_unchecked_mut(dst_universe_index).state };
                    match cell_state {
                        &mut CS {ref mut contents, ..} => {
                            let new_amount;
                            match contents {
                                &mut CellContents::Food(ref mut amount) => {
                                    *amount -= 1;
                                    new_amount = *amount;
                                },
                                _ => { return; }, // If the targeted cell doesn't contain food, abort.
                            }

                            // if this removal depleted the food deposit, set it to empty instead
                            if new_amount == 0 {
                                *contents = CellContents::Empty;
                            }

                            // increment the ant's held food count
                            match entity.state {
                                ES::Ant {ref state, ref mut held_food} => {
                                    if *held_food < ANT_FOOD_CAPACITY {
                                        *held_food += 1
                                    }
                                }
                            }
                        },
                    }
                }
            }
        },
        _ => unreachable!(),
    }
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

fn exec_entity_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

impl SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>> for AntEngine {
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
        for self_action in self_actions { exec_self_action(self_action); }
        for entity_action in entity_actions { exec_entity_action(entity_action); }
    }
}

/// Given a coordinate of the universe, uses state of its cell and the entities that reside in it to determine a color
/// to display on the canvas.  This is called each tick.  The returned value is the color in RGBA.
fn get_color(cell: &Cell<CS>, entity_indexes: &[usize], entity_container: &EntityContainer<CS, ES, MES>) -> [u8; 4] {
    unimplemented!(); // TODO
}

fn main() {
    let conf = UniverseConf {
        iter_cells: false,
        size: 800,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WorldGenerator, cell_mutator, entity_driver);
    let engine: Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>> = Box::new(AntEngine);
    let driver = EmscriptenDriver;
    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, get_color, canvas_render))
    ]);
}
