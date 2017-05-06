//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

use std::cell::Cell as RustCell;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState};
use action::{Action, CellAction, EntityAction};
use util::{get_coords, get_index};
use super::Engine;
use super::grid_iterator::{GridIterator, EntityIterator};

pub trait SerialEngine<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> {
    fn iter_cells(&self) -> RustCell<&mut GridIterator>;

    fn iter_entities(&self, &[Vec<Entity<C, E>>]) -> RustCell<&mut EntityIterator>;

    fn exec_actions(&self, &mut Universe<C, E, CA, EA, Box<SerialEngine<C, E, CA, EA>>>, &[Action<C, E, CA, EA>]);
}

impl<C: CellState, E: EntityState<C>, CA: CellAction<C>, EA: EntityAction<C, E>> Engine<C, E, CA, EA>
    for Box<SerialEngine<C, E, CA, EA>>
{
    fn step<'a>(&'a mut self, mut universe: &'a mut Universe<C, E, CA, EA, Self>) {
        let UniverseConf{view_distance, size, overlapping_entities: _} = universe.conf;

        // iterate over the universe's cells one at a time, applying their state transitions immediately
        let mut grid_iter_cell = self.iter_cells();
        let cell_iterator: &mut &mut GridIterator = grid_iter_cell.get_mut();
        for index in cell_iterator {
            let new_cell = {
                let (cur_x, cur_y) = get_coords(index, size);
                let cur_x = cur_x as isize;
                let cur_y = cur_y as isize;
                let cell_accessor = |x_offset: isize, y_offset: isize| -> Option<&Cell<C>> {
                    let size = size as isize;
                    let req_x = x_offset + cur_x as isize;
                    let req_y = y_offset + cur_y as isize;

                    if (cur_x - req_x).abs() <= view_distance as isize && (cur_y - req_y).abs() <= view_distance as isize &&
                        req_x < size && req_x >= 0 && req_y < size && req_y >= 0
                    {
                        let index = get_index(req_x as usize, req_y as usize, view_distance as usize);
                        Some(&universe.cells[index])
                    } else {
                        None
                    }
                };

                (universe.cell_mutator)(&universe.cells[index], &cell_accessor)
            };
            universe.cells[index] = new_cell;
        }

        // iterate over the universe's entities one at a time, passing their requested actions into the engine's core
        // and applying the results immediately based on its rules
        let mut action_buf = Vec::new(); // TODO: Preserve between iterations
        let mut entity_iter_cell = self.iter_entities(&universe.entities);
        let entity_iterator: &mut &mut EntityIterator = entity_iter_cell.get_mut();
        for (universe_index, entity_index) in entity_iterator {
            let (cur_x, cur_y) = get_coords(universe_index, size);
            let cur_x = cur_x as isize;
            let cur_y = cur_y as isize;
            let size = size as isize;

            let mut action_count = 0;
            {
                let cell_accessor = |x_offset: isize, y_offset: isize| -> Option<&Cell<C>> {
                    let req_x = x_offset + cur_x as isize;
                    let req_y = y_offset + cur_y as isize;

                    if (cur_x - req_x).abs() <= view_distance as isize && (cur_y - req_y).abs() <= view_distance as isize &&
                        req_x < size && req_x >= 0 && req_y < size && req_y >= 0
                    {
                        let index = get_index(req_x as usize, req_y as usize, view_distance as usize);
                        Some(&universe.cells[index])
                    } else {
                        None
                    }
                };

                let entity_accessor = |x_offset: isize, y_offset: isize| -> Option<&Vec<Entity<C, E>>> {
                    let req_x = x_offset + cur_x as isize;
                    let req_y = y_offset + cur_y as isize;

                    if (cur_x - req_x).abs() <= view_distance as isize && (cur_y - req_y).abs() <= view_distance as isize &&
                        req_x < size && req_x >= 0 && req_y < size && req_y >= 0
                    {
                        let index = get_index(req_x as usize, req_y as usize, view_distance as usize);
                        Some(&universe.entities[index])
                    } else {
                        None
                    }
                };

                let action_executor = |action: Action<C, E, CA, EA>| {
                    action_count += 1;
                    if action_buf.len() < action_count {
                        action_buf.push(action);
                    } else {
                        action_buf[action_count - 1] = action;
                    }
                };

                let entity = &universe.entities[universe_index][entity_index];
                // entity.state.transform(&entity_accessor, &cell_accessor, &action_executor); // TODO
            }
            self.exec_actions(&mut universe, action_buf.as_slice().split_at(action_count).0);
        }

        universe.seq += 1;
    }
}
