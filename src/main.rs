//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(test)]

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
use util::{get_coords, get_index};
use driver::{Driver, BasicDriver};
use driver::middleware::{UniverseDisplayer, Delay};

const UNIVERSE_SIZE: usize = 5000;
const ENTITY_COUNT: usize = 500;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
struct OurEntityState {
    energy: u32,
}

impl EntityState<OurCellState> for OurEntityState {}

impl Display for OurEntityState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", 'O')
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
    Create,
    Destroy,
}

impl CellAction<OurCellState> for OurCellAction {}

#[derive(Debug)]
enum OurEntityAction {}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

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
            if cell_x >= 0 && cell_x < UNIVERSE_SIZE as isize && cell_y >= 0 && cell_y < UNIVERSE_SIZE as isize {
                let cell_index = get_index(cell_x as usize, cell_y as usize, UNIVERSE_SIZE);
                match cell_action {
                    &OurCellAction::Create => {
                        universe.cells[cell_index].state = OurCellState::Filled;
                    },
                    &OurCellAction::Destroy => {
                        universe.cells[cell_index].state = OurCellState::Empty;
                    }
                }
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
                    // println!("{}, {}", new_x, new_y);
                    if new_x >= 0 && new_x < UNIVERSE_SIZE as isize && new_y >= 0 && new_y < UNIVERSE_SIZE as isize {
                        let new_index = get_index(new_x as usize, new_y as usize, UNIVERSE_SIZE);
                        // println!("Attempting to move entity {:?} from {}, {} to {}, {}", action.source_uuid, cur_x, cur_y, new_x, new_y);
                        // println!("{:?}", universe.entity_meta);

                        // check to make sure that the entity we're searching for is in its expected location
                        // println!("{:?}", universe.entities[cur_universe_index]);
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
        &Action::EntityAction{action: ref cell_action, x_offset, y_offset} => {
            unimplemented!();
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
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<OurCellState>>, Vec<Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>>, HashMap<Uuid, (usize, usize)>) {
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
            cells.push(Cell{state: OurCellState::Empty});
        }
        let mut entities = vec![Vec::new(); length];

        let mut entity_meta = HashMap::new();
        let mut rng = PcgRng::new_unseeded();
        rng.set_stream(10101010101);
        let origin_entity = Entity::new(
            OurEntityState{energy: 10000},
            OurMutEntityState {rng: Some(rng.clone())}
        );

        for _ in 0..ENTITY_COUNT {
            let index = rng.gen_range(0, UNIVERSE_SIZE);
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

fn our_entity_driver<'a>(
    cur_x: usize,
    cur_y: usize,
    _: &OurEntityState,
    mut_state: &RustCell<OurMutEntityState>,
    _: &[Vec<Entity<OurCellState, OurEntityState, OurMutEntityState>>],
    cells: &[Cell<OurCellState>],
    cell_action_executor: &mut FnMut(OurCellAction, isize, isize),
    self_action_executor: &mut FnMut(SelfAction<OurCellState, OurEntityState, OurEntityAction>),
    entity_action_executor: &mut FnMut(OurEntityAction, isize, isize)
) {
    let mut mut_state_inner = mut_state.take();

    {
        let mut rng = mut_state_inner.rng.as_mut().unwrap();
        let (x_offset, y_offset) = (rng.gen_range(-1, 2), rng.gen_range(-1, 2));
        let action = SelfAction::translate(x_offset, y_offset);
        self_action_executor(action);

        if rng.next_u32() > !(1u32 << 31) {
            let (target_x_offset, target_y_offset) = (-x_offset, -y_offset);
            let (target_x, target_y) = (cur_x as isize + target_x_offset, cur_y as isize + target_y_offset);
            if target_x >= 0 && target_x < UNIVERSE_SIZE as isize && target_y >= 0 && target_y < UNIVERSE_SIZE as isize {
                let target_index = get_index(target_x as usize, target_y as usize, UNIVERSE_SIZE);
                // println!("Queued intent to move from {}, {} to {}, {}", cur_x, cur_y, target_x, target_y);
                let cell_action = match cells[target_index].state {
                    OurCellState::Empty => OurCellAction::Create,
                    OurCellState::Filled => OurCellAction::Destroy,
                };
                cell_action_executor(cell_action, target_x_offset, target_y_offset);
            }
        }
    }

    mut_state.set(mut_state_inner);
}

fn main() {
    let mut conf = universe::UniverseConf::default();
    conf.size = UNIVERSE_SIZE;
    let engine
        : Box<SerialEngine<OurCellState, OurEntityState, OurMutEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>>
    = Box::new(OurEngine {});

    let universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19093929992071),
        our_cell_mutator,
        our_entity_driver,
    );

    let driver = BasicDriver::new();
    driver.init(universe, engine, &mut [/*Box::new(UniverseDisplayer {}), Box::new(Delay(10))*/]);
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
