//! Defines a hybrid client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

use std::borrow::Cow;
use std::ptr;

use minutiae::prelude::*;
use minutiae::server::*;
use minutiae::universe::Into2DIndex;
use uuid::Uuid;

use super::{Client, ClientState, GenClient, Tys};

pub struct HybridClient<T: Tys> where
    T::ServerMessage: ServerMessage<T::Snapshot>,
    T::I: Into2DIndex,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
{
    color_calculator: fn(
        cell: &Cell<T::C>,
        entity_indexes: &[usize],
        entity_container: &EntityContainer<T::C, T::E, T::M, T::I>
    ) -> [u8; 4],
    universe: T::Snapshot,
    universe_size: usize,
    state: ClientState<T::Snapshot, T::ServerMessage>,
    pixbuf: Vec<[u8; 4]>,
}

impl<T: Tys> HybridClient<T> where
    T::ServerMessage: ServerMessage<T::Snapshot>,
    T::I: Into2DIndex,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
{
    fn apply_snap_inner(&mut self, snap: T::Snapshot) {
        self.universe = snap;
        // Generate content for the inner pixel buffer using the universe
        for universe_index in 0..(self.pixbuf.len() / 4) {
            let native_coord: T::I = <T::I as Into2DIndex>::from_2d_index(self.universe_size, universe_index);
            let cell: Cow<Cell<T::C>> = self.universe.get_cell(native_coord).unwrap();;
            let entity_indexes = self.universe.get_entities().get_entities_at(native_coord);
            let new_color: [u8; 4] = (self.color_calculator)(cell.as_ref(), entity_indexes, self.universe.get_entities());

            unsafe {
                let pixel_ptr = self.pixbuf.get_unchecked_mut(universe_index * 4) as *mut [u8; 4];
                // ptr::write(pixel_ptr, [200, 45, 133, 255]);
                // ptr::write(pixel_ptr, new_color.0[0]);
                ptr::write(pixel_ptr, new_color);
            }
            // self.pixbuf[(universe_index * 4) + 0] = new_color.0;
            // self.pixbuf[(universe_index * 4) + 1] = new_color.1;
            // self.pixbuf[(universe_index * 4) + 2] = new_color.2;//, new_color.0[1], new_color.0[2], 255];
        }
    }
}

impl<
    T: Tys<ServerMessage=HybridServerMessage<T>> + 'static
> Client<T> for HybridClient<T> where
    T::ServerMessage: ServerMessage<T::Snapshot>,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
    T::I: Into2DIndex,
    T::I: Into2DIndex,
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
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
    T::I: Into2DIndex,
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
        debug("Creating binary snapshot request message...");
        HybridClientMessage::create_snapshot_request(self.get_uuid())
            .bin_serialize()
            .unwrap()
    }
}

impl<T: Tys> HybridClient<T> where
    <T as Tys>::ServerMessage: ServerMessage<<T as Tys>::Snapshot>,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
    T::I: Into2DIndex,
    T::V: Clone,
    T::Snapshot: Clone,
    T::Snapshot: Universe<T::C, T::E, T::M, Coord=T::I>,
{
    pub fn new(
        universe_size: usize,
        color_calculator: fn(
            cell: &Cell<T::C>,
            entity_indexes: &[usize],
            entity_container: &EntityContainer<T::C, T::E, T::M, T::I>
        ) -> [u8; 4],
    ) -> HybridClient<T> {
        HybridClient {
            universe_size,
            color_calculator,
            state: ClientState::new(),
            universe: T::Snapshot::empty(),
            pixbuf: vec![[0u8, 0u8, 0u8, 255u8]; universe_size * universe_size],
        }
    }
}
