//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(test)]

extern crate rand;
extern crate pcg;
extern crate test;

use std::cell::Cell as RustCell;
use std::fmt::{self, Display, Formatter};

use pcg::PcgRng;
use rand::Rng;

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
use action::{Action, CellAction, EntityAction, TypedAction, SelfAction};
use engine::Engine;
use engine::serial::SerialEngine;
use engine::iterator::{SerialGridIterator, SerialEntityIterator};
use generator::Generator;
use util::{get_coords, get_index};
use driver::{Driver, BasicDriver};
use driver::middleware::{UniverseDisplayer, Delay};

const UNIVERSE_SIZE: usize = 15;

#[derive(Clone)]
enum OurCellState {
    Empty,
    Filled,
}

impl CellState for OurCellState {}

impl Display for OurCellState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let val = match self {
            &OurCellState::Empty => ' ',
            &OurCellState::Filled => 'X',
        };

        write!(formatter, "{}", val)
    }
}

#[derive(Clone)]
struct OurEntityState {
    energy: u32,
}

impl EntityState<OurCellState> for OurEntityState {}

impl Display for OurEntityState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", 'O')
    }
}

#[derive(Clone)]
struct OurMutEntityState {
    rng: Option<PcgRng>,
}

impl MutEntityState for OurMutEntityState {}

impl Default for OurMutEntityState {
    fn default() -> OurMutEntityState {
        OurMutEntityState {
            rng: None,
        }
    }
}

enum OurCellAction {
    Create,
    Destroy,
}

impl CellAction<OurCellState> for OurCellAction {}

enum OurEntityAction {}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

struct OurEngine {}

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
        actions: &[Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>]
    ) {
        for action in actions {
            match action.action {
                TypedAction::SelfAction(ref self_action) => {
                    match self_action {
                        &SelfAction::Translate(x_offset, y_offset) => {
                            let (cur_x, cur_y) = get_coords(action.x_offset as usize, UNIVERSE_SIZE);
                            let new_x = cur_x as isize + x_offset;
                            let new_y = cur_y as isize + y_offset;
                            println!("{}, {}", new_x, new_y);
                            if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                                println!("Removed entity from {}, {}", action.x_offset, action.y_offset);
                                let new_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);
                                let entity = universe.entities[action.x_offset as usize].remove(action.y_offset as usize);
                                universe.entities[new_index].push(entity);
                                println!("Moved entity to {}, {}", new_x, new_y);
                            }
                        }
                        _ => unimplemented!(),
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}

struct OurWorldGenerator(u64);

impl Generator<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction> for OurWorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<OurCellState>>, Vec<Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>>) {
        println!("Generating world...");
        let mut rng = PcgRng::new_unseeded().with_stream(self.0);
        let length = conf.size * conf.size;
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            // let baby_cell = Cell{state: match rng.gen() {
            //     false => OurCellState::Empty,
            //     true => OurCellState::Filled,
            // }};
            // cells.push(baby_cell);
            cells.push(Cell{state: OurCellState::Empty});
        }
        let mut entities = vec![Vec::new(); length];
        let mut rng = PcgRng::new_unseeded();
        rng.set_stream(10101010101);
        let entity = Entity::new(
            OurEntityState{energy: 10000},
            OurMutEntityState {rng: Some(rng)}
        );
        entities[4].push(entity.clone());
        entities[10].push(entity.clone());
        entities[30].push(entity);

        (cells, entities)
    }
}

fn our_cell_mutator<'a>(index: usize, cells: &[Cell<OurCellState>]) -> Option<OurCellState> {
    // Some(match cells[index].state {
    //     OurCellState::Empty => OurCellState::Filled,
    //     OurCellState::Filled => OurCellState::Empty,
    // })
    None
}

fn our_entity_driver<'a>(
    state: &OurEntityState,
    mut_state: &RustCell<OurMutEntityState>,
    entities: &[Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>],
    cells: &[Cell<OurCellState>],
    action_executor: &mut FnMut(Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>),
) {
    let mut mut_state_inner = mut_state.take();
    let action = {
        let rng = mut_state_inner.rng.as_mut().unwrap();
        Action::mut_self(SelfAction::translate(rng.gen_range(-1, 2), rng.gen_range(-1, 2)))
    };
    mut_state.set(mut_state_inner);
    action_executor(action);
}

fn main() {
    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let engine
        : Box<SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>>
    = Box::new(OurEngine {});

    let universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19293929192771),
        our_cell_mutator,
        our_entity_driver,
    );

    let driver = BasicDriver::new();
    driver.init(universe, engine, &mut [Box::new(UniverseDisplayer {}), Box::new(Delay(80))]);
}

#[bench]
fn universe_step(b: &mut test::Bencher) {
    let mut conf = universe::UniverseConf::default();
    conf.size = 1;
    let mut engine: Box<
        SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction,
        OurEntityAction,SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>
    > = Box::new(OurEngine {});

    let mut universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19293929192771),
        our_cell_mutator,
        our_entity_driver,
    );

    b.iter(|| engine.step(&mut universe))
}
