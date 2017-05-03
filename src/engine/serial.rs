//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

use std::iter::Iterator;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use action::Action;
use util::{get_coords, get_middle_entity_coord};
use super::Engine;
use super::grid_iterator::{GridIterator, EntityIterator};

pub trait SerialEngine<C: CellState, E: EntityState<C>> {
    fn iter_cells(&mut self) -> Box<GridIterator>;

    fn iter_entities(&mut self, &[Vec<Entity<C, E>>]) -> Box<EntityIterator>;

    fn supplied_action(&mut self, Action<C, E>);
}

impl<C: CellState, E: EntityState<C>> Engine<C, E> for Box<SerialEngine<C, E>> {
    fn step(&mut self, universe: &mut Universe<C, E, Self>) where Self:Sized {
        let UniverseConf{view_distance, size, overlapping_entities: _} = universe.conf;
        // TODO: Preserve these in between calls somehow.
        let mut neighbor_entity_index_buf = vec![0; view_distance * view_distance];
        let mut neighbor_cell_index_buf = vec![0; view_distance * view_distance];

        // iterate over the universe's cells one at a time, applying their state transitions immediately
        for index in self.iter_cells() {
            let new_cell = {
                let neighbors = universe.get_cell_neighbors(index);
                (universe.cell_mutator)(&universe.cells[index], neighbors)
            };
            universe.cells[index] = new_cell;
        }

        // iterate over the universe's entities one at a time, passing their requested actions into the engine's core
        // and applying the results immediately based on its rules
        for (universe_index, entity_index) in self.iter_entities(&universe.entities) {
            let neighbor_entities = universe.get_entity_neighbors(universe_index, neighbor_entity_index_buf.as_mut_slice());
            let neighbor_cells = universe.get_cell_neighbors(universe_index);
            let entity_coord = get_middle_entity_coord(neighbor_entity_index_buf.as_slice(), view_distance);
            let entity = &universe.entities[entity_coord][entity_index];
            let action = entity.state.transform(&universe.entities, neighbor_entity_index_buf.as_slice(), neighbor_cells);
            self.apply_action(action)
        }

        universe.seq += 1;
    }

    fn apply_action(&mut self, action: Action<C, E>) {
        self.supplied_action(action)
    }
}
