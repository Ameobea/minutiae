//! A place to experiment with the ideas and concepts of the Minuate simulation

use std::cell::Cell as RustCell;

mod universe;
mod cell;
mod entity;
mod action;
mod engine;
mod generator;
mod util;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use action::{Action, CellAction, EntityAction};
use engine::Engine;
use engine::serial::SerialEngine;
use engine::grid_iterator::{GridIterator, EntityIterator};
use generator::Generator;

enum OurCellState {
    Empty,
    Filled,
}

impl CellState for OurCellState {}

struct OurEntityState {
    energy: u32,
}

impl EntityState<OurCellState> for OurEntityState {
    // fn transform<'a, OurCellAction, OurEntityAction>(
    //     &self,
    //     entity_accessor: &Fn(isize, isize) -> Option<&'a Vec<Entity<OurCellState, OurEntityState>>>,
    //     cell_accessor: &Fn(isize, isize) -> Option<&'a Cell<OurCellState>>,
    //     action_executor: &FnMut(Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>)
    // ) {
    //     unimplemented!();
    // }
}

enum OurCellAction {
    Create,
    Destroy,
}

impl CellAction<OurCellState> for OurCellAction {}

enum OurEntityAction {}

impl EntityAction<OurCellState, OurEntityState> for OurEntityAction {}

// TODO: Create included versions of random/ordered iterators and use those instead
struct OurGridIterator {}

impl GridIterator for OurGridIterator {
    fn visit(&mut self) -> Option<usize> {
        unimplemented!();
    }
}

struct OurEntityIterator {}

impl EntityIterator for OurEntityIterator {
    fn visit(&mut self) -> Option<(usize, usize)> {
        unimplemented!();
    }
}

struct OurEngine {
    grid_iterator: Box<GridIterator>,
    entity_iterator: Box<EntityIterator>,
}

impl SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction> for OurEngine {
    fn iter_cells(&self) -> RustCell<&mut GridIterator> {
        // self.grid_iterator.as_ref()
        unimplemented!();
    }

    fn iter_entities(&self, neighbors: &[Vec<Entity<OurCellState, OurEntityState>>]) -> RustCell<&mut EntityIterator> {
        // self.entity_iterator.as_ref()
        unimplemented!();
    }

    fn exec_actions(
        &self,
        universe: &mut Universe
            <OurCellState, OurEntityState, OurCellAction, OurEntityAction, Box<SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction>>>,
        actions: &[Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>]
    ) {
        unimplemented!();
    }
}

struct OurWorldGenerator {}

impl<N: Engine<OurCellState, OurEntityState, OurCellAction, OurEntityAction>> Generator<OurCellState, OurEntityState, OurCellAction, OurEntityAction, N>
    for OurWorldGenerator
{
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<OurCellState>>, Vec<Vec<Entity<OurCellState, OurEntityState>>>) {
        unimplemented!();
    }
}

fn our_cell_mutator<'a>(cell: &Cell<OurCellState>, accessor: &Fn(isize, isize) -> Option<&'a Cell<OurCellState>>) -> Cell<OurCellState> {
    unimplemented!();
}

fn main() {
    use engine::Engine;
    let conf = universe::UniverseConf::default();
    let mut engine: Box<SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction>> = Box::new(OurEngine {
        grid_iterator: Box::new(OurGridIterator {}),
        entity_iterator: Box::new(OurEntityIterator {}),
    });
    let u = universe::Universe::new(conf, &mut OurWorldGenerator{}, Box::new(engine), Box::new(our_cell_mutator));
}
