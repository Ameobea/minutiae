//! Engine that makes use of multiple worker threads to enable entity drivers to be evaluated concurrently.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

use num_cpus;
use uuid::Uuid;

use universe::{CellContainer, ContiguousUniverse, Universe};
use cell::{Cell, CellState};
use entity::{Entity, EntityState, MutEntityState};
use action::{Action, CellAction, SelfAction, EntityAction, OwnedAction};
use engine::Engine;
use container::{EntityContainer, EntitySlot};

type ActionBufs<C, E, CA, EA, I> = (
    Vec<OwnedAction<C, E, CA, EA, I>>,
    usize,
    Vec<OwnedAction<C, E, CA, EA, I>>,
    usize,
    Vec<OwnedAction<C, E, CA, EA, I>>,
    usize,
);

pub struct ParallelEngine<
    C: CellState + Send + 'static,
    E: EntityState<C> + Send + 'static,
    M: MutEntityState + Send + 'static,
    CA: CellAction<C> + Send + 'static,
    EA: EntityAction<C, E> + Send + 'static,
    I: Ord + Copy + Send + 'static,
    CC: CellContainer<C, I>,
    U: Universe<C, E, M, Coord=I>,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>]
    ),
> {
    worker_count: usize,
    // Uses a function trait out of necessity since we have need to do that for the hybrid server.
    exec_actions: F,
    action_buf_rx: Receiver<ActionBufs<C, E, CA, EA, I>>,
    wakeup_senders: Vec<SyncSender<WakeupMessage<C, E, M, CA, EA, I, CC>>>,
    index: Arc<AtomicUsize>,
    recycled_action_bufs: Vec<ActionBufs<C, E, CA, EA, I>>,
    action_buf_buf: Vec<ActionBufs<C, E, CA, EA, I>>,
    __phantom_u: PhantomData<U>,
}

/// Message sent over the wakeup channels containing recycled action buffers and the number of entities that need to be processed
pub struct WakeupMessage<
    C: CellState + Send + 'static,
    E: EntityState<C> + Send + 'static,
    M: MutEntityState + Send + 'static,
    CA: CellAction<C> + Send + 'static,
    EA: EntityAction<C, E> + Send + 'static,
    I: Ord + Copy + Send + 'static,
    CC: CellContainer<C, I>,
> {
    cell_action_buf: Vec<OwnedAction<C, E, CA, EA, I>>,
    self_action_buf: Vec<OwnedAction<C, E, CA, EA, I>>,
    entity_action_buf: Vec<OwnedAction<C, E, CA, EA, I>>,
    entity_count: usize,
    cells_ptr: *const CC,
    entities_ptr: *const EntityContainer<C, E, M, I>,
    index: Arc<AtomicUsize>,
}

unsafe impl<
    C: CellState + Send + 'static,
    E: EntityState<C> + Send + 'static,
    M: MutEntityState + Send + 'static,
    CA: CellAction<C> + Send + 'static,
    EA: EntityAction<C, E> + Send + 'static,
    I: Ord + Copy + Send + 'static,
    CC: CellContainer<C, I>
> Send for WakeupMessage<C, E, M, CA, EA, I, CC> {}

impl<
    C: CellState + Send,
    E: EntityState<C> + Send,
    M: MutEntityState + Send,
    CA: CellAction<C> + Send,
    EA: EntityAction<C, E> + Send,
    I: Ord + Send + Copy + 'static,
    CC: CellContainer<C, I> + Send + 'static,
    U: Universe<C, E, M, Coord=I>,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>]
    )
> ParallelEngine<C, E, M, CA, EA, I, CC, U, F> {
    pub fn new(
        exec_actions: F,
        entity_driver: fn(
            universe_index: I,
            entity: &Entity<C, E, M>,
            entities: &EntityContainer<C, E, M, I>,
            cells: &[Cell<C>],
            cell_action_executor: &mut FnMut(CA, I),
            self_action_executor: &mut FnMut(SelfAction<C, E, EA>),
            entity_action_executor: &mut FnMut(EA, usize, Uuid)
        )
    ) -> Self {
        let cpu_count = num_cpus::get();
        // create a container to hold the senders used to wake up the worker threads
        let mut wakeup_senders = Vec::with_capacity(cpu_count);
        // create a channel over which to receive action buffers from the worker threads
        let (action_buf_tx, action_buf_rx) = sync_channel(cpu_count);

        // spawn worker threads that block waiting for a message to be received indicating that they should start pulling and processing work
        for _ in 0..cpu_count {
            let (wakeup_tx, wakeup_rx) = sync_channel(0);
            wakeup_senders.push(wakeup_tx);
            let action_buf_tx = action_buf_tx.clone();

            thread::spawn(move || {
                let mut entity_index;
                let mut cell_action_count;
                let mut self_action_count;
                let mut entity_action_count;

                // keep blocking and waiting for a wakeup message, then start processing work until it's all completed
                loop {
                    // reset action counts
                    cell_action_count = 0;
                    self_action_count = 0;
                    entity_action_count = 0;

                    let WakeupMessage {
                        mut cell_action_buf, mut self_action_buf, mut entity_action_buf, entity_count, cells_ptr, entities_ptr, index
                    } = wakeup_rx.recv()
                        .expect("Error while receiving work message over channel in worker thread; sender likely gone away!");

                    // convert the current cell and entity pointers into references
                    let entities: &EntityContainer<C, E, M, I> = unsafe {
                        &*(entities_ptr as *const EntityContainer<C, E, M, I>)
                    };
                    // TODO TODO TODO ------------------- THIS IS BAD \/ \/ \/ \/ \/ \/ \/
                    let cells: &Vec<Cell<C>> = unsafe { &*(cells_ptr as *const Vec<Cell<C>>) };

                    // keep processing work as long as there's work left to process
                    loop {
                        entity_index = index.fetch_add(1, Ordering::Relaxed);
                        if entity_index < entity_count {
                            match entities.entities[entity_index] {
                                EntitySlot::Occupied { entity: ref entity_ref, ref universe_index } => {
                                    let mut cell_action_executor = |cell_action: CA, universe_index: I| {
                                        let owned_action = OwnedAction {
                                            source_entity_index: entity_index,
                                            source_uuid: entity_ref.uuid,
                                            action: Action::CellAction {
                                                universe_index,
                                                action: cell_action,
                                            },
                                        };

                                        if cell_action_buf.len() <= cell_action_count {
                                            cell_action_buf.push(owned_action);
                                        } else {
                                            debug_assert!(cell_action_buf.len() > cell_action_count);
                                            unsafe { *cell_action_buf.get_unchecked_mut(cell_action_count) = owned_action };
                                        }
                                        cell_action_count += 1;
                                    };

                                    let mut self_action_executor = |self_action: SelfAction<C, E, EA>| {
                                        let owned_action = OwnedAction {
                                            source_entity_index: entity_index,
                                            source_uuid: entity_ref.uuid,
                                            action: Action::SelfAction(self_action),
                                        };

                                        if self_action_buf.len() <= self_action_count {
                                            self_action_buf.push(owned_action);
                                        } else {
                                            debug_assert!(self_action_buf.len() > self_action_count);
                                            unsafe { *self_action_buf.get_unchecked_mut(self_action_count) = owned_action };
                                        }
                                        self_action_count += 1;
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

                                        if entity_action_buf.len() <= entity_action_count {
                                            entity_action_buf.push(owned_action);
                                        } else {
                                            debug_assert!(entity_action_buf.len() > entity_action_count);
                                            unsafe { *entity_action_buf.get_unchecked_mut(entity_action_count) = owned_action };
                                        }
                                        entity_action_count += 1;
                                    };

                                    // execute the entity driver
                                    entity_driver(
                                        *universe_index,
                                        entity_ref,
                                        entities,
                                        cells,
                                        &mut cell_action_executor,
                                        &mut self_action_executor,
                                        &mut entity_action_executor
                                    );
                                },
                                EntitySlot::Empty(_) => (),
                            }
                        } else {
                            // we've reached the end of the entities and can exit.
                            break;
                        }
                    }

                    // push the buffers back to the main thread over the `action_buf_tx`
                    let msg = (cell_action_buf, cell_action_count, self_action_buf, self_action_count, entity_action_buf, entity_action_count);
                    action_buf_tx.send(msg)
                        .expect("Unable to send action buffers over `action_buf_tx`!");
                }
            });
        }

        // create initial, empty `ActionBufs` to insert into the struct
        let mut recycled_action_bufs = Vec::with_capacity(cpu_count);
        for _ in 0..cpu_count {
            let bufs = (Vec::new(), 0, Vec::new(), 0, Vec::new(), 0);
            recycled_action_bufs.push(bufs);
        }

        // create vector to hold the results from the worker threads without allocating.  Will always have the same length as the
        // number of worker threads (currently the number of CPUs)
        let action_buf_buf = Vec::with_capacity(cpu_count);

        ParallelEngine {
            worker_count: cpu_count,
            exec_actions,
            action_buf_rx,
            wakeup_senders,
            index: Arc::new(AtomicUsize::new(0)),
            recycled_action_bufs,
            action_buf_buf,
            __phantom_u: PhantomData,
        }
    }
}

impl<
    C: CellState + Send + Debug + 'static,
    E: EntityState<C> + Send + Debug + 'static,
    M: MutEntityState + Send + 'static,
    CA: CellAction<C> + Send + Debug + 'static,
    EA: EntityAction<C, E> + Send + Debug + 'static,
    I: Ord + Send + Copy + 'static,
    CC: CellContainer<C, I>,
    U: Universe<C, E, M, Coord=I> + ContiguousUniverse<C, E, M, I, CC>,
    F: Fn(
        &mut U,
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>],
        &[OwnedAction<C, E, CA, EA, I>]
    ),
> Engine<C, E, M, CA, EA, U> for Box<ParallelEngine<C, E, M, CA, EA, I, CC, U, F>> {
    fn step(&mut self, mut universe: &mut U) {
        let &mut ParallelEngine {
            ref index, worker_count,
            ref exec_actions,
            ref action_buf_rx,
            ref mut wakeup_senders,
            ref mut recycled_action_bufs,
            ref mut action_buf_buf,
            ..
        } = &mut **self;

        // TODO: Look into bullying Rust into letting us do without the `Arc` since that's a Heap allocation plus
        // pointer overhead that has to happen every cycle.
        let entity_count = universe.get_entities().len();
        let cells_ptr = universe.get_cell_container() as *const CC;
        let entities_ptr = universe.get_entities() as *const EntityContainer<C, E, M, I>;
        // reset current entity count to 0
        index.store(0, Ordering::Relaxed);

        debug_assert_eq!(wakeup_senders.len(), worker_count);
        // construct wakeup messages to send to all the workers and then send them over to get them doing work
        let mut i = 0;
        {
            for (cell_action_buf, _, self_action_buf, _, entity_action_buf, _) in recycled_action_bufs.drain(..) {
                let msg = WakeupMessage {
                    cell_action_buf: cell_action_buf,
                    self_action_buf: self_action_buf,
                    entity_action_buf: entity_action_buf,
                    cells_ptr: cells_ptr,
                    entities_ptr: entities_ptr,
                    entity_count: entity_count,
                    index: index.clone(),
                };
                unsafe { wakeup_senders.get_unchecked_mut(i) }.send(msg)
                    .expect("Unable to send wakeup message to worker thread!");
                i += 1;
            }
        }

        debug_assert_eq!(action_buf_buf.len(), 0);
        // collect the results from the worker threads
        for _ in 0..worker_count {
            let bufs = action_buf_rx.recv()
                .expect("Error while receiving action buffers from worker thread; thread probably died.");
            action_buf_buf.push(bufs);
        }
        debug_assert_eq!(action_buf_buf.len(), worker_count);

        // execute all the queued actions once all workers have finished.
        let exec_actions = exec_actions;
        let mut i = 0;
        for (
            mut cell_action_buf, cell_action_count, mut self_action_buf, self_action_count, mut entity_action_buf, entity_action_count
        ) in action_buf_buf.drain(..) {
            // since we're re-using the buffers without clearing out old values for performance, set their lengths manually
            let (real_cell_len, real_self_len, real_entity_len) = (cell_action_buf.len(), self_action_buf.len(), entity_action_buf.len());
            unsafe {
                cell_action_buf.set_len(cell_action_count);
                self_action_buf.set_len(self_action_count);
                entity_action_buf.set_len(entity_action_count);
            }

            // evaluate all pending actions, allowing the engine to handle any conflicts
            exec_actions(&mut universe, &cell_action_buf, &self_action_buf, &entity_action_buf);

            // recycle the action buffers to avoid having to re-allocate them later
            unsafe {
                cell_action_buf.set_len(real_cell_len);
                self_action_buf.set_len(real_self_len);
                entity_action_buf.set_len(real_entity_len);
            }
            recycled_action_bufs.push((cell_action_buf, 0, self_action_buf, 0, entity_action_buf, 0));

            i += 1;
        }
        debug_assert_eq!(i as usize, worker_count);
    }
}
