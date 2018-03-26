//! Defines a thin client designed to receive raw data corresponding to the color of pixels.  This is the most
//! computationally cheap client to implement but suffers from large bandwidth requiremenets in situations where
//! there's a large amount of change in the universe in between ticks.

use std::marker::PhantomData;
use std::ptr;

use minutiae::server::*;
use minutiae::util::Color;
use uuid::Uuid;

use super::{Client, ClientState, GenClient, Tys, debug};

pub struct ThinClient<T: Tys> {
    pub universe: Vec<[u8; 4]>,
    pub state: ClientState<Vec<Color>, ThinServerMessage>,
    __phantom_t: PhantomData<T>,
}

impl<T: Tys> ThinClient<T> {
    pub fn new(universe_size: usize) -> Self {
        ThinClient {
            // TODO: Investigate if this is necessary
            universe: vec![[0u8, 0u8, 0u8, 255u8]; universe_size * universe_size * 4],
            state: ClientState::new(),
            __phantom_t: PhantomData,
        }
    }

    pub fn apply_diff(&mut self, diff: Diff) {
        debug_assert!(diff.universe_index < self.universe.len());
        unsafe {
            let ptr = self.universe.get_unchecked_mut(diff.universe_index).as_mut_ptr() as *mut [u8; 3];
            ptr::write(ptr, diff.color.0);
        };
    }

    fn apply_snap_inner(&mut self, snap: Vec<Color>) {
        debug("Received snapshot from server... attempting to apply it.");
        debug_assert_eq!(self.universe.len(), snap.len() / 4);
        for (i, color) in snap.iter().enumerate() {
            self.apply_diff(Diff {universe_index: i, color: *color});
        }
    }
}

impl<
    T: Tys<
        Snapshot=Vec<Color>,
        ServerMessage=ThinServerMessage
    > + 'static
> Client<T> for ThinClient<T> where
    ThinServerMessage: ServerMessage<T::Snapshot>
{
    fn handle_message(&mut self, message: ThinServerMessage) {
        match message.contents {
            ThinServerMessageContents::Diff(diffs) => {
                // apply all diffs contained in the message
                for diff in diffs {
                    self.apply_diff(diff);
                }
            },
            ThinServerMessageContents::Snapshot(snap) => self.apply_snap_inner(snap),
        }
    }

    fn apply_snap(&mut self, snap: T::Snapshot) {
        self.apply_snap_inner(snap);
    }

    fn get_state(&mut self) -> &mut ClientState<Vec<Color>, ThinServerMessage> {
        &mut self.state
    }
}

impl<
    T: Tys<
        Snapshot=Vec<Color>,
        ServerMessage=ThinServerMessage
    > + 'static
> GenClient for ThinClient<T> where T::ServerMessage: ServerMessage<T::Snapshot> {
    fn get_pixbuf_ptr(&self) -> *const u8 {
        self.universe.as_ptr() as *const u8
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
