//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

use std::collections::HashSet;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, OwnedAction, CellAction, EntityAction};
use util::get_coords;
use super::Engine;
use super::iterator::{GridIterator, EntityIterator};

pub trait SerialEngine<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> {
    fn iter_cells(&self, &[Cell<C>]) -> CI;

    fn iter_entities(&self, &[Vec<Entity<C, E, M>>]) -> EI;

    fn exec_actions(&self, &mut Universe<C, E, M, CA, EA>, &[OwnedAction<C, E, CA, EA>]);
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> Engine<C, E, M, CA, EA> for Box<SerialEngine<C, E, M, CA, EA, CI, EI>> {
    // #[inline(never)]
    fn step<'a>(&'a mut self, mut universe: &'a mut Universe<C, E, M, CA, EA>) {
        let UniverseConf{view_distance: _, size, overlapping_entities: _} = universe.conf;

        // iterate over the universe's cells one at a time, applying their state transitions immediately
        let cell_iterator: &mut GridIterator = &mut self.iter_cells(&universe.cells);
        for index in cell_iterator {
            match (universe.cell_mutator)(index, &universe.cells) {
                Some(new_state) => universe.cells[index].state = new_state,
                None => (),
            }
        }

        // iterate over the universe's entities one at a time, passing their requested actions into the engine's core
        // and applying the results immediately based on its rules
        let mut action_buf = Vec::with_capacity(universe.average_actions_per_cycle);
        for universe_index in universe.entity_meta.iter() {
            let universe_index = *universe_index;
            for entity_index in 0..universe.entities[universe_index].len() {
                let (cur_x, cur_y) = get_coords(universe_index, size);

                let mut action_executor = |action: Action<C, E, CA, EA>| {
                    let owned_action = OwnedAction {
                        source_universe_index: universe_index,
                        source_entity_index: entity_index,
                        action: action,
                    };

                    action_buf.push(owned_action);
                };

                let entity = &universe.entities[universe_index][entity_index];
                (universe.entity_driver)(
                    cur_x, cur_y, &entity.state, &entity.mut_state, &universe.entities, &universe.cells, &mut action_executor
                );
            }
        }

        // update the universe with new estimated actions/cycle
        universe.seq += 1;
        universe.average_actions_per_cycle = (universe.total_actions * action_buf.len()) / universe.seq;

        // evaluate all pending actions simultaneously, allowing the engine to handle any conflicts
        self.exec_actions(&mut universe, &action_buf);
    }
}
