//! A simulation engine that simulates all changes to the universe sequentially.  This is the most simple
//! engine but doens't take advantage of any possible benifits from things like multithreading.

use universe::{Universe};
use cell::CellState;
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, OwnedAction, CellAction, SelfAction, EntityAction};

use super::Engine;
use super::iterator::EntityIterator;

use uuid::Uuid;

pub trait SerialEngine<
    C: CellState + 'static,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    EI: EntityIterator<C, E, M>,
    U: Universe<C, E, M>,
> {
    fn iter_entities(&self, &U) -> EI;

    fn exec_actions(
        &self,
        &mut U,
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>],
        &[OwnedAction<C, E, CA, EA>]
    );

    fn drive_entity(
        &mut self,
        universe_index: usize,
        entity: &Entity<C, E, M>,
        universe: &U,
        cell_action_executor: &mut FnMut(CA, usize),
        self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
        entity_action_executor: &mut FnMut(EA, usize, Uuid)
    );
}

impl<
    C: CellState + 'static,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    EI: EntityIterator<C, E, M>,
    U: Universe<C, E, M>,
> Engine<C, E, M, CA, EA, U> for Box<SerialEngine<C, E, M, CA, EA, EI, U>> {
    // #[inline(never)]
    fn step<'a>(&'a mut self, mut universe: &'a mut U) {
        // iterate over the universe's entities one at a time, passing their requested actions into the engine's core
        // and applying the results immediately based on its rules
        // TODO: Implement preallocation and preallocation metrics
        let mut cell_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
        let mut self_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
        let mut entity_action_buf: Vec<OwnedAction<C, E, CA, EA>> = Vec::new();
        for (entity_ref, entity_index, universe_index) in universe.get_entities().iter() {
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

            (**self).drive_entity(
                universe_index,
                &entity_ref,
                universe,
                &mut cell_action_executor,
                &mut self_action_executor,
                &mut entity_action_executor
            );
        }

        // evaluate all pending actions simultaneously, allowing the engine to handle any conflicts
        self.exec_actions(
            &mut universe,
            &cell_action_buf,
            &self_action_buf,
            &entity_action_buf
        );
    }
}
