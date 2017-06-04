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
use util::Color;

/// Defines a message that transmits diff-based data representing how the universe's representation as pixel data
/// changed between two ticks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinServerMessage {
    pub seq: u32,
    pub contents: ThinServerMessageContents,
}

impl ServerMessage<Vec<Color>> for ThinServerMessage {
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

pub struct ColorServer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    pub universe_len: usize,
    pub colors: RwLock<Vec<Color>>,
    pub color_calculator: fn(&Cell<C>, entity_indexes: &[usize], entity_container: &EntityContainer<C, E, M>) -> Color,
    pub seq: Arc<AtomicU32>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> ColorServer<C, E, M> {
    pub fn new(
        universe_size: usize, color_calculator: fn(
            &Cell<C>, entity_indexes: &[usize], entity_container: &EntityContainer<C, E, M>
        ) -> Color
    ) -> Self { // boxed so we're sure it doesn't move and we can pass pointers to it around
        let universe_len = universe_size * universe_size;
        ColorServer {
            universe_len,
            colors: RwLock::new(vec![Color([0, 0, 0]); universe_len]),
            color_calculator,
            seq: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl<
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static,
    CA: CellAction<C> + 'static, EA: EntityAction<C, E> + 'static,
> ServerLogic<C, E, M, CA, EA, ThinServerMessage, ThinClientMessage> for ColorServer<C, E, M> {
    fn tick(&mut self, universe: &mut Universe<C, E, M, CA, EA>) -> Option<ThinServerMessage> {
        // TODO: Create an option for making this parallel because it's a 100% parallelizable task
        let mut diffs = Vec::new();
        let mut colors = self.colors.write().expect("Unable to lock colors vector for writing!");
        for i in 0..self.universe_len {
            let cell = unsafe { universe.cells.get_unchecked(i) };
            let entity_indexes = universe.entities.get_entities_at(i);

            let new_color = (self.color_calculator)(cell, entity_indexes, &universe.entities);
            let mut last_color = unsafe { colors.get_unchecked_mut(i) };
            if &new_color != last_color {
                // color for that coordinate has changed, so add a diff to the diff buffer and update `last_colors`
                /*self.*/diffs.push(Diff {universe_index: i, color: new_color.clone()});
                (*last_color) = new_color;
            }
        }

        // create a `ServerMessage` out of the diffs, serialize/compress it, and broadcast it to all connected clients
        Some(ThinServerMessage {
            seq: self.seq.load(Ordering::Relaxed),
            contents: ThinServerMessageContents::Diff(diffs),
        })
    }

    fn handle_client_message(
        server: &mut Server<C, E, M, CA, EA, ThinServerMessage, ThinClientMessage, Self>, client_message: &ThinClientMessage
    ) -> Option<ThinServerMessage> {
        match client_message.content {
            ThinClientMessageContent::SendSnapshot => {
                // create the snapshot by cloning the colors from the server.
                let snap: Vec<Color> = (*server).logic.colors.read().unwrap().clone();
                Some(ThinServerMessage {
                    seq: (*server).get_seq(),
                    contents: ThinServerMessageContents::Snapshot(snap),
                })
            },
            _ => None, // TOOD
        }
    }
}
