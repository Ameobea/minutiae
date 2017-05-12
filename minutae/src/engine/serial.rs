//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

// use rayon::prelude::*;

use universe::{Universe, UniverseConf};
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, OwnedAction, CellAction, SelfAction, EntityAction};

use super::Engine;
use super::iterator::{GridIterator, EntityIterator};

use uuid::Uuid;

pub trait SerialEngine<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> {
    fn iter_cells(&self, &[Cell<C>]) -> CI;

    fn iter_entities(&self, &[Vec<Entity<C, E, M>>]) -> EI;

    fn exec_actions(&self, &mut Universe<C, E, M, CA, EA>, &[OwnedAction<C, E, CA, EA>], &[OwnedAction<C, E, CA, EA>], &[OwnedAction<C, E, CA, EA>]);
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> Engine<C, E, M, CA, EA> for Box<SerialEngine<C, E, M, CA, EA, CI, EI>> {
    // #[inline(never)]
    fn step<'a>(&'a mut self, mut universe: &'a mut Universe<C, E, M, CA, EA>) {
        let UniverseConf{view_distance: _, size: _, iter_cells} = universe.conf;

        // iterate over the universe's cells one at a time, applying their state transitions immediately
        if iter_cells {
            let cell_iterator: &mut GridIterator = &mut self.iter_cells(&universe.cells);
            for index in cell_iterator {
                match (universe.cell_mutator)(index, &universe.cells) {
                    Some(new_state) => universe.cells[index].state = new_state,
                    None => (),
                }
            }
        }

        // iterate over the universe's entities one at a time, passing their requested actions into the engine's core
        // and applying the results immediately based on its rules
        // TODO: Implement preallocation and preallocation metrics
        let mut cell_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
        let mut self_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
        let mut entity_action_buf: Vec<OwnedAction<C, E, CA, EA>> = Vec::new();
        for (entity_ref, entity_index, universe_index) in universe.entities.iter() {
            let mut cell_action_executor = |cell_action: CA, universe_index: usize| {
                let owned_action = OwnedAction {
                    source_entity_index: entity_index,
                    source_uuid: entity_ref.uuid,
                    action: Action::CellAction {
                        universe_index: universe_index,
                        action: cell_action,
                    },
                };

                cell_action_buf.push(owned_action);
            };

            let mut self_action_executor = |self_action: SelfAction<C, E, EA>| {
                let owned_action = OwnedAction {
                    source_entity_index: entity_index,
                    source_uuid: entity_ref.uuid,
                    action: Action::SelfAction(self_action),
                };

                self_action_buf.push(owned_action);
            };

            let mut entity_action_executor = |entity_action: EA, target_entity_index: usize, target_uuid: Uuid| {
                let owned_action = OwnedAction {
                    source_entity_index: entity_index,
                    source_uuid: entity_ref.uuid,
                    action: Action::EntityAction {
                        action: entity_action,
                        target_entity_index: target_entity_index,
                        target_uuid: target_uuid,
                    },
                };

                entity_action_buf.push(owned_action);
            };

            (universe.entity_driver)(
                universe_index, &entity_ref, &universe.entities, &universe.cells,
                &mut cell_action_executor, &mut self_action_executor, &mut entity_action_executor
            );
        }

        // update the universe with new estimated actions/cycle
        universe.seq += 1;
        // universe.average_actions_per_cycle = (universe.total_actions * action_buf.len()) / universe.seq;

        // evaluate all pending actions simultaneously, allowing the engine to handle any conflicts
        self.exec_actions(&mut universe, &cell_action_buf, &self_action_buf, &entity_action_buf);
    }
}
