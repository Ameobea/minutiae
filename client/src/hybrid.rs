//! Defines a hybrid client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

use minutiae::universe::Universe;
use minutiae::cell::CellState;
use minutiae::entity::{EntityState, MutEntityState};
use minutiae::action::{CellAction, EntityAction};
use minutiae::server::*;

use super::{Client, ClientState};

pub struct HybridClient<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> {
    universe_length: usize,
    universe: Universe<C, E, M, CA, EA>,
    state: ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>>,
    pixbuf: Vec<[u8; 4]>,
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> Client<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> for HybridClient<C, E, M, CA, EA, V> {
    fn handle_message(&mut self, message: HybridServerMessage<C, E, M, CA, EA, V>) {
        match message.contents {
            HybridServerMessageContents::Event(evts) => for e in evts { e.apply(&mut self.universe); },
            HybridServerMessageContents::Snapshot(snap) => self.apply_snap(snap),
            _ => unreachable!(),
        }
    }

    fn apply_snap(&mut self, snap: HybridServerSnapshot<C, E, M>) {
        let (cells, entities) = snap;
        self.universe.cells = cells;
        self.universe.entities = entities;
    }

    fn get_pixbuf_ptr(&self) -> *const u8 {
        self.pixbuf.as_ptr() as *const u8
    }

    fn get_state(&mut self) -> &mut ClientState<HybridServerSnapshot<C, E, M>, HybridServerMessage<C, E, M, CA, EA, V>> {
        &mut self.state
    }
}

impl<
    C: CellState + HybParam, E: EntityState<C> + HybParam, M: MutEntityState + HybParam, CA: CellAction<C> + HybParam,
    EA: EntityAction<C, E> + HybParam, V: Event<C, E, M, CA, EA> + HybParam
> HybridClient<C, E, M, CA, EA, V> {
    pub fn new(universe_size: usize) -> Self {
        let universe_length = universe_size * universe_size;
        HybridClient {
            universe_length,
            state: ClientState::new(),
            universe: Universe::uninitialized(universe_size),
            pixbuf: vec![[0u8; 4]; universe_length],
        }
    }
}
