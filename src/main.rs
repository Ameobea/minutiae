//! A place to experiment with the ideas and concepts of the Minuate simulation

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
use engine::iterator::{SerialGridIterator, SerialEntityIterator};
use generator::Generator;

enum OurCellState {
    Empty,
    Filled,
}

impl CellState for OurCellState {}

struct OurEntityState {
    energy: u32,
}

impl EntityState<OurCellState> for OurEntityState {}

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
        universe: &mut Universe
            <OurCellState, OurEntityState, OurCellAction, OurEntityAction, Box
                <SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>>>,
        actions: &[Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>]
    ) {
        unimplemented!();
    }
}

struct OurWorldGenerator {}

impl<N: Engine<OurCellState, OurEntityState, OurCellAction, OurEntityAction>> Generator<OurCellState, OurEntityState, OurCellAction, OurEntityAction, N>
    for OurWorldGenerator
{
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<OurCellState>>, Vec<Vec<Entity<OurCellState, OurEntityState>>>, Vec<usize>) {
        unimplemented!();
    }
}

fn our_cell_mutator<'a>(cell: &Cell<OurCellState>, accessor: &Fn(isize, isize) -> Option<&'a Cell<OurCellState>>) -> Cell<OurCellState> {
    unimplemented!();
}

fn our_entity_driver<'a>(
    entity: &Entity<OurCellState, OurEntityState>,
    entity_accessor: &Fn(isize, isize) -> Option<&'a Vec<Entity<OurCellState, OurEntityState>>>,
    cell_accessor: &Fn(isize, isize) -> Option<&'a Cell<OurCellState>>,
    action_executor: &FnMut(Action<OurCellState, OurEntityState, OurCellAction, OurEntityAction>),
) {
    unimplemented!();
}

fn main() {
    use engine::Engine;
    let conf = universe::UniverseConf::default();
    let mut engine: Box<SerialEngine<OurCellState, OurEntityState, OurCellAction, OurEntityAction, SerialGridIterator, SerialEntityIterator<OurCellState, OurEntityState>>> = Box::new(OurEngine {});
    let u = universe::Universe::new(
        conf,
        &mut OurWorldGenerator{},
        Box::new(engine),
        Box::new(our_cell_mutator),
        Box::new(our_entity_driver),
    );
}
