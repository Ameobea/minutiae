//! Defines a communication protocol between a server and a client where the client only maintains an image of
//! the universe without actually managing any of its state.  Messages contain the difference between two ticks
//! at a pixel-level and are compressed to save bandwidth.
//!
//! They are best suited to situations where the server logic is very computationally expensive and the differences
//! between ticks are not very large (large differences cause large bandwidth usage).

use std::cmp::Ordering;

use futures::Future;
use futures::future::ok;
use uuid::Uuid;

use super::*;
use universe::Universe;
use util::Color;
#[allow(unused_imports)]
use prelude::*;

/// Defines a message that transmits diff-based data representing how the universe's representation as pixel data
/// changed between two ticks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinServerMessage {
    pub seq: u32,
    pub contents: ThinServerMessageContents,
}

impl ServerMessage<Vec<Color>> for ThinServerMessage {
    fn get_seq(&self) -> u32 { self.seq }

    fn is_snapshot(&self) -> bool {
        if let ThinServerMessageContents::Snapshot(_) = self.contents {
            true
        } else {
            false
        }
    }

    fn get_snapshot(self) -> Option<Vec<Color>> {
        match self.contents {
            ThinServerMessageContents::Snapshot(snap) => Some(snap),
            _ => None,
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

#[derive(Clone)]
pub struct ColorServer<
    T: Tys,
    I: ExactSizeIterator<Item=T::I>,
> {
    pub colors: Arc<RwLock<Vec<Color>>>,
    pub color_calculator: fn(
        &Cell<T::C>,
        entity_indexes: &[usize],
        entity_container: &EntityContainer<T::C, T::E, T::M, T::I>
    ) -> Color,
    pub iterator: fn(T::I, T::I) -> I,
    pub start_index: T::I,
    pub end_index: T::I,
}

impl<
    T: Tys,
    I: ExactSizeIterator<Item=T::I> + Clone + 'static,
> ColorServer<T, I> where T::I: Clone + Copy + Ord + Clone + 'static {
    pub fn new(
        color_calculator: fn(
            &Cell<T::C>,
            entity_indexes: &[usize],
            entity_container: &EntityContainer<T::C, T::E, T::M, T::I>,
        ) -> Color,
        iterator: fn(T::I, T::I) -> I,
        default_start_index: T::I,
        default_end_index: T::I,
    ) -> Self {
        let grid_size = iterator(default_start_index, default_end_index).len();

        ColorServer {
            colors: Arc::new(RwLock::new(vec![Color([0, 0, 0,]); grid_size])),
            color_calculator,
            iterator,
            start_index: default_start_index,
            end_index: default_end_index,
        }
    }
}

impl<
    T: Tys<
        ServerMessage=ThinServerMessage,
        ClientMessage=ThinClientMessage,
    >,
    I: ExactSizeIterator<Item=T::I> + Clone + 'static,
> ServerLogic<T> for ColorServer<T, I> where
    T::I: Clone + Copy + Ord + Clone + Sync + 'static,
    ThinServerMessage: ServerMessage<<T as Tys>::Snapshot>,
    T::Snapshot: Clone,
    T::V: Clone,
{
    fn tick(&mut self, seq: u32, universe: &mut T::U) -> Option<Vec<ThinServerMessage>> {
        // TODO: Create an option for making this parallel because it's a 100% parallelizable task
        let mut diffs = Vec::new();
        let mut colors = self.colors.write().expect("Unable to lock colors vector for writing!");

        for (i, coord) in (self.iterator)(self.start_index, self.end_index).into_iter().enumerate() {
            let entity_indexes = universe.get_entities().get_entities_at(coord);
            // let cell = unsafe { universe.get_cell_unchecked(coord) };
            let cell = universe.get_cell(coord).unwrap();

            let new_color = (self.color_calculator)(cell.as_ref(), entity_indexes, universe.get_entities());
            // let mut last_color = unsafe { colors.get_unchecked_mut(i) };
            let mut last_color = colors.get_mut(i).unwrap();
            if &new_color != last_color {
                // color for that coordinate has changed, so add a diff to the diff buffer and update `last_colors`
                /*self.*/diffs.push(Diff { universe_index: i, color: new_color.clone() });
                (*last_color) = new_color;
            }
        }

        // create a `ServerMessage` out of the diffs, serialize/compress it, and broadcast it to all connected clients
        Some(vec![ThinServerMessage {
            seq,
            contents: ThinServerMessageContents::Diff(diffs),
        }])
    }

    fn handle_client_message(
        &mut self,
        seq: u32,
        client_message: ThinClientMessage
    ) -> Box<Future<Item=Option<ThinServerMessage>, Error=!>> {
        match client_message.content {
            ThinClientMessageContent::SendSnapshot => {
                // create the snapshot by cloning the colors from the server.
                let snap: Vec<Color> = self.colors.read().unwrap().clone();
                box ok(Some(ThinServerMessage {
                    seq,
                    contents: ThinServerMessageContents::Snapshot(snap),
                }))
            },
            _ => box ok(None), // TOOD
        }
    }
}

#[bench]
/// Tests the process of encoding a server message as binary and compressing it.
fn server_message_encode(b: &mut test::Bencher) {
    let message = ThinServerMessage {
        seq: 100012,
        contents: ThinServerMessageContents::Diff(vec![Diff{universe_index: 100, color: Color([9u8, 144u8, 88u8])}; 100000]),
    };

    b.bytes = serialized_size(&message).unwrap();

    b.iter(|| {
        message.bin_serialize().unwrap()
    });

    let bin: Vec<u8> = message.clone().bin_serialize().unwrap();
    let decoded = ThinServerMessage::bin_deserialize(&bin).unwrap();
    assert_eq!(message, decoded);
}

#[bench]
/// Tests the process of decompressing a compressed binary representation of a message and making it back into a message.
fn server_message_decode(b: &mut ::test::Bencher) {
    let message = ThinServerMessage {
        seq: 100012,
        contents: ThinServerMessageContents::Diff(vec![Diff{universe_index: 100, color: Color([9u8, 144u8, 88u8])}; 100000]),
    };
    let serialized = message.bin_serialize().unwrap();

    b.bytes = serialized_size(&message).unwrap();

    b.iter(|| {
        ThinServerMessage::bin_deserialize(&serialized).unwrap()
    });

    let decoded = ThinServerMessage::bin_deserialize(&serialized).unwrap();
    assert_eq!(message, decoded);
}

#[test]
fn clientmessage_serialize_deserialize() {
    let msg = ThinClientMessage{
        client_id: Uuid::new_v4(),
        content: ThinClientMessageContent::CellAction{action_id: 8u8, universe_index: 999},
    };
    let serialized: Vec<u8> = msg.clone().bin_serialize().unwrap();
    let deserialized = ThinClientMessage::bin_deserialize(&serialized).unwrap();
    assert_eq!(msg, deserialized);
}
