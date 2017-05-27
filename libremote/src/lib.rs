//! Minutiae libremote.  See README.md for additional information.

#![feature(test)]

extern crate test;
extern crate bincode;
extern crate flate2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use std::fmt::Debug;
use std::io::BufReader;
use std::cmp::{PartialOrd, Ord, Ordering};

use bincode::{serialize, deserialize, serialize_into, serialized_size, Infinite};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;
use uuid::Uuid;

/// A message that is passed over the websocket between the server and a client.
pub trait Message : Sized {
    /// Given the UUID of the client, wraps the payload into a `ClietMessage` and serializes it
    /// in binary format without compressing it
    fn serialize(&self) -> Result<Vec<u8>, String>;

    /// Decodes a binary-encoded message.
    fn deserialize(data: &[u8]) -> Result<Self, String>;
}

// glue to implement `Message` for everything by default where it's possible
impl<T> Message for T where T:Debug + PartialEq + Eq + Sized + Send + serde::Serialize, for<'de> T: serde::Deserialize<'de> {
    fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self, Infinite).map_err(|_| String::from("Unable to serialize message."))
    }

    fn deserialize(data: &[u8]) -> Result<Self, String> {
        bincode::deserialize(data).map_err(|_| String::from("Unable to deserialize message."))
    }
}

/// A message transmitted from the server to one or more clients.  Contains a sequence number used to order messages
/// and determine if any have been missed.
pub trait ServerMessage<S> : Message + Ord {
    fn get_seq(&self) -> u32;
    fn get_snapshot(self) -> Result<S, Self>;
}

pub trait ClientMessage : Message {
    fn get_client_id(&self) -> Uuid;
    fn create_snapshot_request(client_id: Uuid) -> Self;
}

/// Implements serialization/deserialization for `self` that runs the compressed buffer through deflate compression/
/// decompression in order to reduce the size of the serialized buffer.
pub trait CompressedMessage : Sized + Send + PartialEq + serde::Serialize {
    /// Encodes the message in binary format, compressing it in the process.
    fn do_serialize(&self) -> Result<Vec<u8>, String> {
        println!("Size of raw binary: {}", serialized_size(self));
        let mut compressed = Vec::with_capacity(serialized_size(self) as usize);
        {
            let mut encoder = DeflateEncoder::new(&mut compressed, Compression::Fast);
            serialize_into(&mut encoder, self, Infinite)
                .map_err(|_| String::from("Error while serializing compressed message."))?;
            encoder.finish().map_err(|err| format!("Unable to finish the encoder: {:?}", err))?;
        }
        // println!("Size of compressed binary: {}", compressed.len());
        Ok(compressed)
    }

    /// Decodes and decompresses a binary-encoded message.
    fn do_deserialize(data: &[u8]) -> Result<Self, String> where for<'de> Self: serde::Deserialize<'de> {
        let mut decoder = DeflateDecoder::new(BufReader::new(data));
        bincode::deserialize_from(&mut decoder, Infinite)
            .map_err(|err| format!("Error deserializing decompressed binary into compressed message: {:?}", err))
    }
}

impl<'d, T> CompressedMessage for T where T:Debug + Eq + CompressedMessage, for<'de> Self: serde::Deserialize<'de> {}

/// All messages that are passed between the server and clients are of this form.  Each message is accompanied by a sequence
/// number that is used to ensure that they're applied in order.  There will never be a case in which sequence numbers are
/// skipped; if a client misses a message or receives an out-of-order message, it should be stored until the missing one is
/// received or a message should be sent requesting a re-broadcast.
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

#[bench]
/// Tests the process of encoding a server message as binary and compressing it.
fn server_message_encode(b: &mut test::Bencher) {
    let message = ThinServerMessage {
        seq: 100012,
        contents: ThinServerMessageContents::Diff(vec![Diff{universe_index: 100, color: Color([9u8, 144u8, 88u8])}; 100000]),
    };

    b.bytes = serialized_size(&message);

    b.iter(|| {
        message.serialize().unwrap()
    });

    let bin: Vec<u8> = message.clone().serialize().unwrap();
    let decoded = ThinServerMessage::deserialize(&bin).unwrap();
    assert_eq!(message, decoded);
}

#[bench]
/// Tests the process of decompressing a compressed binary representation of a message and making it back into a message.
fn server_message_decode(b: &mut test::Bencher) {
    let message = ThinServerMessage {
        seq: 100012,
        contents: ThinServerMessageContents::Diff(vec![Diff{universe_index: 100, color: Color([9u8, 144u8, 88u8])}; 100000]),
    };
    let serialized = message.serialize().unwrap();

    b.bytes = serialized_size(&message);

    b.iter(|| {
        ThinServerMessage::deserialize(&serialized).unwrap()
    });

    let decoded = ThinServerMessage::deserialize(&serialized).unwrap();
    assert_eq!(message, decoded);
}

#[test]
fn clientmessage_serialize_deserialize() {
    let msg = ThinClientMessage{
        client_id: Uuid::new_v4(),
        content: ThinClientMessageContent::CellAction{action_id: 8u8, universe_index: 999},
    };
    let serialized: Vec<u8> = msg.clone().serialize().unwrap();
    let deserialized = ThinClientMessage::deserialize(&serialized).unwrap();
    assert_eq!(msg, deserialized);
}
