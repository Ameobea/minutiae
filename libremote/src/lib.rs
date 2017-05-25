//! Minutiae libremote.  See README.md for additional information.

#![feature(test)]

extern crate test;
extern crate bincode;
extern crate flate2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use std::io::Write;
use std::cmp::{PartialOrd, Ord, Ordering};

use bincode::{serialize, deserialize, serialize_into, serialized_size, Infinite};
use flate2::Compression;
use flate2::write::{DeflateEncoder, DeflateDecoder};
use uuid::Uuid;

/// A message that is passed over the websocket between the server and a client.
pub trait Message : PartialEq + Eq + Sized + Send {
    fn get_seq(&self) -> u32;
    fn serialize(&self) -> Result<Vec<u8>, String>;
    fn deserialize(&[u8]) -> Result<Self, String>;
}

/// All messages that are passed between the server and clients are of this form.  Each message is accompanied by a sequence
/// number that is used to ensure that they're applied in order.  There will never be a case in which sequence numbers are
/// skipped; if a client misses a message or receives an out-of-order message, it should be stored until the missing one is
/// received or a message should be sent requesting a re-broadcast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinServerMessage {
    pub seq: u32,
    pub contents: ThinServerMessageContents,
}

impl Message for ThinServerMessage {
    fn get_seq(&self) -> u32 { self.seq }

    /// Encodes the message in binary format, compressing it in the process.
    fn serialize(&self) -> Result<Vec<u8>, String> {
        // println!("Size of raw binary: {}", serialized_size(self));
        let mut compressed = Vec::with_capacity(serialized_size(self) as usize);
        {
            let mut encoder = DeflateEncoder::new(&mut compressed, Compression::Fast);
            serialize_into(&mut encoder, self, Infinite).map_err(|_| String::from("Error while serializing `ThinServerMessage`."))?;
            encoder.finish().map_err(|err| format!("Unable to finish the encoder: {:?}", err))?;
        }
        // println!("Size of compressed binary: {}", compressed.len());
        Ok(compressed)
    }

    /// Decodes and decompresses a binary-encoded message.
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let buf = Vec::with_capacity(data.len());
        let mut decoder = DeflateDecoder::new(buf);
        decoder.write_all(data)
            .map_err(|err| format!("Unable to decompress binary `ThinServerMessage`: {:?}", err))?;
        let decompressed = decoder.finish()
            .map_err(|err| format!("Error deserializing decompressed binary into `ThinServerMessage`: {:?}", err))?;
        deserialize(&decompressed)
            .map_err(|err| format!("Error deserializing decompressed binary into `ThinServerMessage`: {:?}", err))
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
    pub seq: u32,
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

impl Message for ThinClientMessage {
    fn get_seq(&self) -> u32 { self.seq }

    /// Given the UUID of the client, wraps the payload into a `ClietMessage` and serializes it
    /// in binary format without compressing it
    fn serialize(&self) -> Result<Vec<u8>, String> {
        serialize(self, Infinite).map_err(|err| format!("Unable to serialize `ThinClientMessage`: {:?}", err))
    }

    /// Decodes a binary-encoded message.
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        deserialize(data)
            .map_err(|err| format!("Error deserializing decompressed binary into `ThinClientMessage`: {:?}", err))
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
        seq: 11111,
        client_id: Uuid::new_v4(),
        content: ThinClientMessageContent::CellAction{action_id: 8u8, universe_index: 999},
    };
    let serialized: Vec<u8> = msg.clone().serialize().unwrap();
    let deserialized = ThinClientMessage::deserialize(&serialized).unwrap();
    assert_eq!(msg, deserialized);
}
