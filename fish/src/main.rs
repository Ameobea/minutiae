//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(alloc_system, conservative_impl_trait, integer_atomics, slice_patterns, test)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate alloc_system;
extern crate rand;
extern crate pcg;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;
extern crate uuid;
extern crate ws;

extern crate minutiae;

use std::fmt::{self, Display, Formatter};

use pcg::PcgRng;
use rand::Rng;
use uuid::Uuid;

use minutiae::universe::{Universe, UniverseConf};
use minutiae::container::EntityContainer;
use minutiae::cell::{Cell, CellState};
use minutiae::entity::{Entity, EntityState, MutEntityState};
use minutiae::action::{Action, CellAction, EntityAction, OwnedAction, SelfAction};
use minutiae::engine::Engine;
#[cfg(not(target_os = "emscripten"))]
use minutiae::engine::parallel::ParallelEngine;
#[cfg(target_os = "emscripten")]
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::{SerialGridIterator};
use minutiae::generator::Generator;
use minutiae::util::{calc_offset, get_coords, get_index, iter_visible, manhattan_distance};
use minutiae::driver::{Driver, BasicDriver};
use minutiae::driver::middleware::{Middleware, MinDelay};
use minutiae::server::{self, Color};
#[cfg(target_os = "emscripten")]
use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};

mod engine;
use engine::*;
mod entity_logic;
use entity_logic::*;

// :ok_hand:
#[cfg(target_os = "emscripten")]
const UNIVERSE_SIZE: usize = 800;
#[cfg(target_os = "emscripten")]
const FISH_COUNT: usize = 2366;
#[cfg(target_os = "emscripten")]
const PREDATOR_COUNT: usize = 0;
#[cfg(target_os = "emscripten")]
const VIEW_DISTANCE: usize = 2;
// there's a one in `this` chance of spawning a food cluster each tick
#[cfg(target_os = "emscripten")]
const FOOD_SPAWN_RARITY: usize = 2;
// this number of food cells are spawned (minus overlaps)
#[cfg(target_os = "emscripten")]
const FOOD_SPAWN_COUNT: usize = 300;
#[cfg(target_os = "emscripten")]
const FOOD_SPAWN_RADIUS: isize = 40;

#[cfg(not(target_os = "emscripten"))]
// const TICK_DELAY_MS: u64 = 16;
#[cfg(not(target_os = "emscripten"))]
const UNIVERSE_SIZE: usize = 800;
#[cfg(not(target_os = "emscripten"))]
const FISH_COUNT: usize = 20342;
#[cfg(not(target_os = "emscripten"))]
const PREDATOR_COUNT: usize = 3;
#[cfg(not(target_os = "emscripten"))]
const VIEW_DISTANCE: usize = 1;
// there's a one in `this` chance of spawning a food cluster each tick
#[cfg(not(target_os = "emscripten"))]
const FOOD_SPAWN_RARITY: usize = 24;
// this number of food cells are spawned (minus overlaps)
#[cfg(not(target_os = "emscripten"))]
const FOOD_SPAWN_COUNT: usize = 2226;
#[cfg(not(target_os = "emscripten"))]
const FOOD_SPAWN_RADIUS: isize = 35;

// const SCHOOL_SPACING: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OurCellState {
    Water,
    Food,
}

impl CellState for OurCellState {}

impl Display for OurCellState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let val = match *self {
            OurCellState::Water => ' ',
            OurCellState::Food => '\'',
        };

        write!(formatter, "{}", val)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OurEntityState {
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
        let val = match *self {
            OurEntityState::Predator{..} => 'V',
            OurEntityState::Fish{..} => '^',
        };
        write!(formatter, "{}", val)
    }
}

#[derive(Copy, Serialize, Deserialize)]
pub struct OurMutEntityState {
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
pub enum OurCellAction {
    Eat, // The only thing that we can really do to the world right now is eat food
}

impl CellAction<OurCellState> for OurCellAction {}

#[derive(Debug)]
pub enum OurEntityAction {
    EatFish,
    MakeBaby,
    SetVector(i8, i8),
}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

#[cfg(not(target_os = "emscripten"))]
type OurEngineType = Box<
        ParallelEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction,
        OurEntityAction, SerialGridIterator>
    >;

#[cfg(target_os = "emscripten")]
type OurEngineType = Box<
    SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction,
    OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>
>;

struct OurWorldGenerator(u64);

impl Generator<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction> for OurWorldGenerator {
    fn gen(
        &mut self, conf: &UniverseConf
    ) -> (
        Vec<Cell<OurCellState>>,
        Vec<Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>>,
    ) {
        println!("Generating world...");
        let length = conf.size * conf.size;
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            // populate the world with water cells
            cells.push(Cell{state: OurCellState::Water});
        }

        let mut entities = vec![Vec::new(); length];
        let mut rng = PcgRng::new_unseeded().with_stream(self.0);
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

fn our_cell_mutator(_: usize, _: &[Cell<OurCellState>]) -> Option<OurCellState> { None }

/// Adds a step onto the end of each simulation cycle that has a chance of spawning some food into the world for the fish to eat
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
    use minutiae::universe;

    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    #[cfg(target_os = "emscripten")]
    let engine: OurEngineType = Box::new(OurEngine {});
    #[cfg(not(target_os = "emscripten"))]
    let engine = Box::new(
        ParallelEngine::new(SerialGridIterator::new(UNIVERSE_SIZE * UNIVERSE_SIZE), exec_actions, our_entity_driver)
    );

    let universe = universe::Universe::new(
        conf,
        &mut OurWorldGenerator(19093929992071),
        our_cell_mutator,
        our_entity_driver,
    );

    #[cfg(target_os = "emscripten")]
    {
        EmscriptenDriver.init(universe, engine, &mut [
            Box::new(FoodSpawnerMiddleware::new()),
            Box::new(CanvasRenderer::new()),
        ]);
    }

    #[cfg(not(target_os = "emscripten"))]
    {
        fn calc_color(
            cell: &Cell<OurCellState>,
            entity_indexes: &[usize],
            entity_container: &EntityContainer<OurCellState, OurEntityState, OurMutEntityState>
        ) -> Color {
            if !entity_indexes.is_empty() {
                for i in entity_indexes {
                    if let OurEntityState::Predator{..} = *unsafe { &entity_container.get(*i).state } {
                        return Color([233, 121, 78]);
                    }
                }
                Color([12, 24, 222])
            } else {
                match cell.state {
                    OurCellState::Water => Color([0, 0, 0]),
                    OurCellState::Food => Color([12, 231, 2]),
                }
            }
        }

        let server_logic = server::ColorServer::new(UNIVERSE_SIZE, calc_color);
        let seq = server_logic.seq.clone();
        let server = server::Server::new(UNIVERSE_SIZE, "0.0.0.0:7037", server_logic, seq);
        let driver = BasicDriver;
        driver.init(universe, engine, &mut [
            // Box::new(UniverseDisplayer {}),
            // Box::new(Delay(TICK_DELAY_MS)),
            Box::new(MinDelay::from_tps(59.97)),
            Box::new(FoodSpawnerMiddleware::new()),
            Box::new(server),
        ]);
    }
}

#[bench]
fn universe_step_parallel(b: &mut test::Bencher) {
    use minutiae::universe;

    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let mut engine = Box::new(
        ParallelEngine::new(SerialGridIterator::new(UNIVERSE_SIZE * UNIVERSE_SIZE), exec_actions, our_entity_driver)
    );

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
    })
}
