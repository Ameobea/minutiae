//! Defines a hybrid client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

use minutiae::prelude::*;
use minutiae::server::*;
use uuid::Uuid;

use super::{Client, ClientState, GenClient, Tys};

pub struct HybridClient<T: Tys> where T::ServerMessage: ServerMessage<T::Snapshot> {
    universe: T::Snapshot,
    state: ClientState<T::Snapshot, T::ServerMessage>,
    pixbuf: Vec<[u8; 4]>,
}

impl<T: Tys> HybridClient<T> where T::ServerMessage: ServerMessage<T::Snapshot> {
    fn apply_snap_inner(&mut self, snap: T::Snapshot) {
        self.universe = snap;
    }
}

impl<
    T: Tys<ServerMessage=HybridServerMessage<T>> + 'static
> Client<T> for HybridClient<T> where
    T::ServerMessage: ServerMessage<T::Snapshot>,
    T::Snapshot: Clone,
    T::V: Clone,
{
    fn handle_message(
        &mut self,
        message: T::ServerMessage
    ) {
        match message.contents {
            HybridServerMessageContents::Event(evts) => for e in evts { e.apply(&mut self.universe); },
            HybridServerMessageContents::Snapshot(snap) => self.apply_snap_inner(snap),
            _ => unreachable!(),
        }
    }

    fn apply_snap(&mut self, snap: T::Snapshot) {
        self.apply_snap_inner(snap);
    }

    fn get_state(&mut self) -> &mut ClientState<<T as Tys>::Snapshot, T::ServerMessage> {
        &mut self.state
    }
}

impl<
    T: Tys<ServerMessage=HybridServerMessage<T>> + 'static
> GenClient for HybridClient<T> where
    <T as Tys>::ServerMessage: ServerMessage<<T as Tys>::Snapshot>,
    T::V: Clone,
    T::Snapshot: Clone
{
    fn get_pixbuf_ptr(&self) -> *const u8 {
        self.pixbuf.as_ptr() as *const u8
    }

    fn get_uuid(&self) -> Uuid {
        self.state.uuid
    }

    fn handle_bin_message(&mut self, msg: &[u8]) {
        Client::handle_binary_message(self, msg);
    }

    fn create_snapshot_request(&self) -> Vec<u8> {
        ThinClientMessage::create_snapshot_request(self.get_uuid())
            .bin_serialize()
            .unwrap()
    }
}

impl<T: Tys> HybridClient<T> where
    <T as Tys>::ServerMessage: ServerMessage<<T as Tys>::Snapshot>,
    T::V: Clone,
    T::Snapshot: Clone,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
{
    pub fn new(universe_size: usize) -> HybridClient<T> {
        HybridClient {
            state: ClientState::new(),
            universe: T::Snapshot::empty(),
            pixbuf: vec![[0u8; 4]; universe_size * universe_size],
        }
    }
}
