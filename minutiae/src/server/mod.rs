//! Sets up code for communicating changes in universe state with remote clients.

use std::{mem, ptr, thread};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use std::fmt::Debug;
use std::io::BufReader;
use std::cmp::{PartialOrd, Ord, Ordering as CmpOrdering};

use bincode::{self, serialize, deserialize, serialize_into, serialized_size, Infinite};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use universe::Universe;
use container::EntityContainer;
use cell::{Cell, CellState};
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use engine::Engine;
use driver::middleware::Middleware;

#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use self::server::*;

mod thin;
pub use self::thin::*;
mod hybrid;
pub use self::hybrid::*;
mod fat;
pub use self::fat::*;

/// A message that is passed over the websocket between the server and a client.
pub trait Message : Sized {
    /// Given the UUID of the client, wraps the payload into a `ClietMessage` and serializes it
    /// in binary format without compressing it
    fn serialize(&self) -> Result<Vec<u8>, String>;

    /// Decodes a binary-encoded message.
    fn deserialize(data: &[u8]) -> Result<Self, String>;
}

// glue to implement `Message` for everything by default where it's possible
impl<T> Message for T where T:Debug + PartialEq + Eq + Sized + Send + Serialize, for<'de> T: Deserialize<'de> {
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
pub trait CompressedMessage : Sized + Send + PartialEq + Serialize {
    /// Encodes the message in binary format, compressing it in the process.
    fn do_serialize(&self) -> Result<Vec<u8>, String> {
        println!("Size of raw binary: {}", serialized_size(self));
        let mut compressed = Vec::with_capacity(serialized_size(self) as usize);
        {
            let mut encoder = DeflateEncoder::new(&mut compressed, Compression::Default);
            serialize_into(&mut encoder, self, Infinite)
                .map_err(|_| String::from("Error while serializing compressed message."))?;
            encoder.finish().map_err(|err| format!("Unable to finish the encoder: {:?}", err))?;
        }
        // println!("Size of compressed binary: {}", compressed.len());
        Ok(compressed)
    }

    /// Decodes and decompresses a binary-encoded message.
    fn do_deserialize(data: &[u8]) -> Result<Self, String> where for<'de> Self: Deserialize<'de> {
        let mut decoder = DeflateDecoder::new(BufReader::new(data));
        bincode::deserialize_from(&mut decoder, Infinite)
            .map_err(|err| format!("Error deserializing decompressed binary into compressed message: {:?}", err))
    }
}

impl<'d, T> CompressedMessage for T where T:Debug + Eq + CompressedMessage, for<'de> Self: Deserialize<'de> {}

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
