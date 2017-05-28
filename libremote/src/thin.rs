//! Defines a communication protocol between a server and a client where the client only maintains an image of
//! the universe without actually managing any of its state.  Messages contain the difference between two ticks
//! at a pixel-level and are compressed to save bandwidth.
//!
//! They are best suited to situations where the server logic is very computationally expensive and the differences
//! between ticks are not very large (large differences cause large bandwidth usage).

use std::cmp::Ordering;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use super::{ServerMessage, ClientMessage};

/// Defines a message that transmits diff-based data representing how the universe's representation as pixel data
/// changed between two ticks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinServerMessage {
    pub seq: u32,
    pub contents: ThinServerMessageContents,
}

impl<'de> ServerMessage<Vec<Color>> for ThinServerMessage {
    fn get_seq(&self) -> u32 { self.seq }

    fn get_snapshot(self) -> Result<Vec<Color>, Self> {
        match self.contents {
            ThinServerMessageContents::Snapshot(snap) => Ok(snap),
            _ => Err(self),
        }
    }
}

impl PartialOrd for ThinServerMessage {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.seq.cmp(&rhs.seq))
    }
}

impl Ord for ThinServerMessage {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.seq.cmp(&rhs.seq)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinServerMessageContents {
    Diff(Vec<Diff>),
    Snapshot(Vec<Color>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub [u8; 3]);

/// Encodes the difference between two different steps of a simulation.  Currently simply contains a universe index and
/// and the object that is visible there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    pub universe_index: usize,
    pub color: Color,
}

/// A message sent from a client to the server
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinClientMessage {
    pub client_id: Uuid,
    pub content: ThinClientMessageContent,
}

/// The payload of a message sent from a client to the server
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinClientMessageContent {
    Retransmit(u32), // a request to retransmit a missed diff packet
    SendSnapshot, // a request to send a snapshot of the universe as it currently exists
    // some custom action applied to a particular universe coordinate that should be handled by the server
    CellAction {
        action_id: u8,
        universe_index: usize,
    },
}

impl ClientMessage for ThinClientMessage {
    fn get_client_id(&self) -> Uuid { self.client_id }

    fn create_snapshot_request(client_id: Uuid) -> Self {
        ThinClientMessage {
            client_id,
            content: ThinClientMessageContent::SendSnapshot,
        }
    }
}
