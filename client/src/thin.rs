//! Defines a thin client designed to receive raw data corresponding to the color of pixels.  This is the most
//! computationally cheap client to implement but suffers from large bandwidth requiremenets in situations where
//! there's a large amount of change in the universe in between ticks.

use std::ptr;

use uuid::Uuid;

use minutiae::server::*;
use super::{Client, ClientState, debug};

pub struct ThinClient {
    pub universe: Vec<[u8; 4]>,
    pub state: ClientState<Vec<Color>, ThinServerMessage>,
}

impl ThinClient {
    pub fn new(universe_size: usize) -> Self {
        ThinClient {
            universe: vec![[0u8, 0u8, 0u8, 255u8]; universe_size * universe_size * 4],
            state: ClientState::new(),
        }
    }

    pub fn apply_diff(&mut self, diff: Diff) {
        debug_assert!(diff.universe_index < self.universe.len());
        unsafe {
            let ptr = self.universe.get_unchecked_mut(diff.universe_index).as_mut_ptr() as *mut [u8; 3];
            ptr::write(ptr, diff.color.0);
        };
    }
}

impl Client<Vec<Color>, ThinServerMessage> for ThinClient {
    fn handle_message(&mut self, message: ThinServerMessage) {
        match message.contents {
            ThinServerMessageContents::Diff(diffs) => {
                // apply all diffs contained in the message
                for diff in diffs {
                    self.apply_diff(diff);
                }
            },
            ThinServerMessageContents::Snapshot(snap) => self.apply_snap(snap),
        }
    }

    fn apply_snap(&mut self, snap: Vec<Color>) {
        debug("Received snapshot from server... attempting to apply it.");
        debug_assert_eq!(self.universe.len(), snap.len() / 4);
        for (i, color) in snap.iter().enumerate() {
            self.apply_diff(Diff {universe_index: i, color: *color});
        }
    }

    fn get_state(&mut self) -> &mut ClientState<Vec<Color>, ThinServerMessage> {
        &mut self.state
    }

    fn get_pixbuf_ptr(&self) -> *const u8 {
        self.universe.as_ptr() as *const u8
    }
}
