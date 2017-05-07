//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, TypedAction, CellAction, EntityAction};
use util::{get_coords, get_index};
use super::Engine;
use super::iterator::{GridIterator, EntityIterator};

pub trait SerialEngine<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> {
    fn iter_cells(&self, &[Cell<C>]) -> CI;

    fn iter_entities(&self, &[Vec<Entity<C, E, M>>]) -> EI;

    fn exec_actions(&self, &mut Universe<C, E, M, CA, EA>, &[Action<C, E, CA, EA>]);
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> Engine<C, E, M, CA, EA> for Box<SerialEngine<C, E, M, CA, EA, CI, EI>> {
    // #[inline(never)]
    fn step<'a>(&'a mut self, mut universe: &'a mut Universe<C, E, M, CA, EA>) {
        let UniverseConf{view_distance, size, overlapping_entities: _} = universe.conf;

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
        let mut action_buf = Vec::new(); // TODO: Preserve between iterations
        let entity_iterator: &mut EntityIterator<C, E, M> = &mut self.iter_entities(&universe.entities);
        while let Some((universe_index, entity_index)) = entity_iterator.visit(&universe.entities) {
            let (cur_x, cur_y) = get_coords(universe_index, size);
            let cur_x = cur_x as isize;
            let cur_y = cur_y as isize;
            let size = size as isize;

            let mut action_count = 0;
            {
                let mut action_executor = |mut action: Action<C, E, CA, EA>| {
                    // set the x and y offsets to universe and entity index for self actions
                    match action.action {
                        TypedAction::SelfAction(_) => {
                            action.x_offset = universe_index as isize;
                            action.y_offset = entity_index as isize;
                        },
                        _ => (),
                    }

                    action_count += 1;
                    if action_buf.len() < action_count {
                        action_buf.push(action);
                    } else {
                        action_buf[action_count - 1] = action;
                    }
                };

                let mut entity = &universe.entities[universe_index][entity_index];
                (universe.entity_driver)(
                    &entity.state, &entity.mut_state, &universe.entities, &universe.cells, &mut action_executor
                );
            }
            self.exec_actions(&mut universe, action_buf.as_slice().split_at(action_count).0);
        }

        universe.seq += 1;
    }
}
