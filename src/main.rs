//! A place to experiment with the ideas and concepts of the Minuate simulation

#![feature(test)]

extern crate rand;
extern crate pcg;
extern crate test;

use std::fmt::{self, Display, Formatter};

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
use entity::{Entity, EntityState};
use action::{Action, CellAction, EntityAction};
use engine::Engine;
use engine::serial::SerialEngine;
use engine::iterator::{SerialGridIterator, SerialEntityIterator};
use generator::Generator;
use driver::{Driver, BasicDriver};
use driver::middleware::{UniverseDisplayer, Delay};

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

enum OurCellAction {
    Create,
    Destroy,
}

impl CellAction<OurCellState> for OurCellAction {}

enum OurEntityAction {}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

struct OurEngine {}

impl SerialEngine
    <OurCellState, OurEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>
for OurEngine {
    fn iter_cells(&self, cells: &[Cell<OurCellState>]) -> SerialGridIterator {
        SerialGridIterator::new(cells.len())
    }

    fn iter_entities<'a>(
        &self, entities: &'a [Vec<Entity<OurCellState, OurEntityState>>]
    ) -> SerialEntityIterator<OurCellState, OurEntityState> {
        SerialEntityIterator::new(entities.len())
    }

    fn exec_actions(
        &self,
        universe: &mut Universe<OurCellState, OurEntityState, OurCellAction, OurEntityAction>,
        actions: &[Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>]
    ) {
        unimplemented!();
    }
}

struct OurWorldGenerator(u64);

impl Generator<OurCellState, OurEntityState, OurCellAction, OurEntityAction> for OurWorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<OurCellState>>, Vec<Vec<Entity<OurCellState, OurEntityState>>>) {
        println!("Generating world...");
        let mut rng = pcg::PcgRng::new_unseeded().with_stream(self.0);
        let length = conf.size * conf.size;
        let mut cells = Vec::with_capacity(length);
        for _ in 0..length {
            let baby_cell = Cell{state: match rng.gen() {
                false => OurCellState::Empty,
                true => OurCellState::Filled,
            }};
            cells.push(baby_cell);
        }
        let entities = vec![Vec::new(); length];

        (cells, entities)
    }
}

fn our_cell_mutator<'a>(index: usize, cells: &[Cell<OurCellState>]) -> Option<OurCellState> {
    Some(match cells[index].state {
        OurCellState::Empty => OurCellState::Filled,
        OurCellState::Filled => OurCellState::Empty,
    })
}

fn our_entity_driver<'a>(
    _: &Entity<OurCellState, OurEntityState>,
    _: &Fn(isize, isize) -> Option<&'a Vec<Entity<OurCellState, OurEntityState>>>,
    _: &Fn(isize, isize) -> Option<&'a Cell<OurCellState>>,
    _: &FnMut(Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>),
) {
    // unimplemented!();
}

fn main() {
    let mut conf = universe::UniverseConf::default();
    conf.size = 25;
    let engine
        : Box<SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>>
    = Box::new(OurEngine {});

    let universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19293929192771),
        our_cell_mutator,
        Box::new(our_entity_driver),
    );

    let driver = BasicDriver::new();
    driver.init(universe, engine, &mut [Box::new(UniverseDisplayer {}), Box::new(Delay(100))]);
}

#[bench]
fn universe_step(b: &mut test::Bencher) {
    let mut conf = universe::UniverseConf::default();
    conf.size = 1;
    let mut engine
        : Box<SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>>
    = Box::new(OurEngine {});

    let mut universe = Universe::new(
        conf,
        &mut OurWorldGenerator(19293929192771),
        our_cell_mutator,
        Box::new(our_entity_driver),
    );

    b.iter(|| engine.step(&mut universe))
}
