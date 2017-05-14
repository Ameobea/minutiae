//! Engine that makes use of multiple worker threads to enable entity drivers to be evaluated concurrently.

use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use num_cpus;
use smallvec::SmallVec;
use uuid::Uuid;

use universe::Universe;
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, CellAction, SelfAction, EntityAction, OwnedAction};
use engine::Engine;
use container::{EntityContainer, EntitySlot};
use super::iterator::{GridIterator, EntityIterator};

pub trait ParallelEngine<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>,
    EA: EntityAction<C, E>, CI: GridIterator, EI: EntityIterator<C, E, M>
> {
    fn iter_cells(&self, &[Cell<C>]) -> CI;

    fn iter_entities(&self, &[Vec<Entity<C, E, M>>]) -> EI;

    fn exec_actions(&self, &mut Universe<C, E, M, CA, EA>, &[OwnedAction<C, E, CA, EA>], &[OwnedAction<C, E, CA, EA>], &[OwnedAction<C, E, CA, EA>]);
}

impl<
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static, CA: CellAction<C> + 'static,
    EA: EntityAction<C, E> + 'static, CI: GridIterator, EI: EntityIterator<C, E, M>
> Engine<C, E, M, CA, EA> for Box<ParallelEngine<C, E, M, CA, EA, CI, EI>> where
    C:Send, E:Send, M:Send, CA:Send, EA:Send, CA: ::std::fmt::Debug, EA: ::std::fmt::Debug, C: ::std::fmt::Debug, E: ::std::fmt::Debug
{
    fn step(&mut self, mut universe: &mut Universe<C, E, M, CA, EA>) {
        // iterate over the universe's cells one at a time, applying their state transitions immediately
        // TODO: Consider parallel processing here
        if universe.conf.iter_cells {
            let cell_iterator: &mut GridIterator = &mut self.iter_cells(&universe.cells);
            for index in cell_iterator {
                match (universe.cell_mutator)(index, &universe.cells) {
                    Some(new_state) => universe.cells[index].state = new_state,
                    None => (),
                }
            }
        }

        // create channels to collect the queued actions from the worker threads
        // let (cell_action_tx, cell_action_rx) = channel(8);
        // let (self_action_tx, self_action_rx) = channel(8);
        // let (entity_action_tx, entity_action_rx) = channel(8);
        // and vectors to hold the collected values

        // TODO: Look into bullying Rust into letting us do without the `Arc` since that's a Heap allocation plus
        // pointer overhead that has to happen every cycle.
        let index = Arc::new(AtomicUsize::new(0));
        let entity_count = universe.entities.entities.len();

        // stack-allocate space for 4 handles with any extras spilling over into the heap.
        // let mut handles: SmallVec<[JoinHandle<[Vec<OwnedAction<C, E, CA, EA>>; 3]>; 4]> = SmallVec::new();
        let mut handles = Vec::new();
        let entity_driver = universe.entity_driver;
        let cells_ptr = &universe.cells as *const Vec<Cell<C>> as usize;
        let entities_ptr = &universe.entities as *const EntityContainer<C, E, M> as usize;

        for _ in 0..num_cpus::get() {
            let index_clone = index.clone();
            let handle = thread::spawn(move || -> (Vec<OwnedAction<C, E, CA, EA>>, Vec<OwnedAction<C, E, CA, EA>>, Vec<OwnedAction<C, E, CA, EA>>) {
                // allocate space for this worker's generated actions
                let mut cell_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
                let mut self_action_buf: Vec<OwnedAction<C, E, CA, EA>>   = Vec::new();
                let mut entity_action_buf: Vec<OwnedAction<C, E, CA, EA>> = Vec::new();
                let entities: &EntityContainer<C, E, M> = unsafe { &*(entities_ptr as *const EntityContainer<C, E, M>) };
                let cells: &Vec<Cell<C>> = unsafe { &*(cells_ptr as *const Vec<Cell<C>>) };
                let mut entity_index;

                loop {
                    entity_index = index_clone.fetch_add(1, Ordering::Relaxed);
                    if entity_index < entity_count {
                        match entities.entities[entity_index] {
                            EntitySlot::Occupied{entity: ref entity_ref, universe_index} => {
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

                                // execute the entity driver
                                entity_driver(
                                    universe_index, entity_ref, entities, cells,
                                    &mut cell_action_executor, &mut self_action_executor, &mut entity_action_executor
                                );
                            },
                            EntitySlot::Empty(_) => (),
                        }
                    } else {
                        // we've reached the end of the entities and can exit.
                        break;
                    }
                }

                (cell_action_buf, self_action_buf, entity_action_buf)
            });

            handles.push(handle);;
        }

        // collect the results from the worker threads
        // let mut actions: SmallVec<[[Vec<OwnedAction<C, E, CA, EA>>; 3]; 3]> = SmallVec::new();
        let mut actions = Vec::new();
        for handle in handles {
            let actions_array = handle.join().expect("Error while joining worker thread!");
            // println!("{:?}", actions_array);
            actions.push(actions_array);
        }

        // execute all the queued actions once all workers have finished.
        for (cell_action_buf, self_action_buf, entity_action_buf) in actions {
            universe.seq += 1;
            // universe.average_actions_per_cycle = (universe.total_actions * action_buf.len()) / universe.seq;

            // evaluate all pending actions simultaneously, allowing the engine to handle any conflicts
            self.exec_actions(&mut universe, &cell_action_buf, &self_action_buf, &entity_action_buf);
        }
    }
}
