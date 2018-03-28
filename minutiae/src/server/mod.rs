//! Sets up code for communicating changes in universe state with remote clients.

use std::cmp::{PartialOrd, Ord};
use std::fmt::Debug;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicU32;

use bincode::{self, serialize_into, serialized_size};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;
use futures::Future;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use prelude::*;

#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use self::server::*;

#[cfg(any(feature = "server", feature = "client"))]
mod thin;
#[cfg(any(feature = "server", feature = "client"))]
pub use self::thin::*;
#[cfg(any(feature = "server", feature = "client"))]
mod hybrid;
#[cfg(any(feature = "server", feature = "client"))]
pub use self::hybrid::*;
#[cfg(any(feature = "server", feature = "client"))]
mod fat;
#[cfg(any(feature = "server", feature = "client"))]
pub use self::fat::*;

pub trait Tys : Clone + Copy + Send {
    type C: CellState;
    type E: EntityState<Self::C>;
    type M: MutEntityState;
    type CA: CellAction<Self::C>;
    type EA: EntityAction<Self::C, Self::E>;
    type I: Ord + Copy;
    type U: Universe<Self::C, Self::E, Self::M, Coord=Self::I>;
    type V: Event<Self> = ();

    #[cfg(not(any(feature = "thin", feature = "hybrid", feature = "fat")))]
    type Snapshot = ();

    #[cfg(feature = "thin")]
    type Snapshot = Vec<Color>;

    #[cfg(feature = "hybrid")]
    type Snapshot = Self::U;

    #[cfg(feature = "fat")]
    type Snapshot = (); // TODO

    #[cfg(not(any(feature = "thin", feature = "hybrid", feature = "fat")))]
    type ServerMessage: ServerMessage<Self::Snapshot> = ();

    #[cfg(feature = "thin")]
    type ServerMessage: ServerMessage<Self::Snapshot> = ThinServerMessage;

    #[cfg(feature = "hybrid")]
    type ServerMessage: ServerMessage<Self::Snapshot> = HybridServerMessage<Self::Snapshot>;

    #[cfg(feature = "fat")]
    type ServerMessage: ServerMessage<Self::Snapshot> = (); // TODO
}

/// A message that is passed over the websocket between the server and a client.
pub trait Message : Sized {
    /// Given the UUID of the client, wraps the payload into a `ClietMessage` and serializes it
    /// in binary format without compressing it
    fn bin_serialize(&self) -> Result<Vec<u8>, String>;

    /// Decodes a binary-encoded message.
    fn bin_deserialize(data: &[u8]) -> Result<Self, String>;
}

// glue to implement `Message` for everything by default where it's possible
impl<T> Message for T where T:Debug + PartialEq + Eq + Sized + Send + Serialize, for<'de> T: Deserialize<'de> {
    fn bin_serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|err| format!("Unable to serialize message: {:?}", err))
    }

    fn bin_deserialize(data: &[u8]) -> Result<Self, String> {
        bincode::deserialize(data).map_err(|err| format!("Unable to deserialize message: {:?}", err))
    }
}

/// A message transmitted from the server to one or more clients.  Contains a sequence number used to order messages
/// and determine if any have been missed.
pub trait ServerMessage<S> : Message + Ord {
    fn get_seq(&self) -> u32;

    fn is_snapshot(&self) -> bool;

    fn get_snapshot(self) -> Option<S>;
}

pub trait ClientMessage : Message {
    fn get_client_id(&self) -> Uuid;
    fn create_snapshot_request(client_id: Uuid) -> Self;
}

/// Implements serialization/deserialization for `self` that runs the compressed buffer through deflate compression/
/// decompression in order to reduce the size of the serialized buffer.
pub trait CompressedMessage: Sized + Send + PartialEq + Serialize {
    /// Encodes the message in binary format, compressing it in the process.
    fn do_serialize(&self) -> Result<Vec<u8>, String> {
        println!("Size of raw binary: {}", serialized_size(self).unwrap());
        let mut compressed = Vec::with_capacity(serialized_size(self).unwrap() as usize);
        {
            let mut encoder = DeflateEncoder::new(&mut compressed, Compression::default());
            serialize_into(&mut encoder, self)
                .map_err(|_| String::from("Error while serializing compressed message."))?;
            encoder.finish().map_err(|err| format!("Unable to finish the encoder: {:?}", err))?;
        }
        // println!("Size of compressed binary: {}", compressed.len());
        Ok(compressed)
    }

    /// Decodes and decompresses a binary-encoded message.
    fn do_deserialize(data: &[u8]) -> Result<Self, String> where for<'de> Self: Deserialize<'de> {
        let mut decoder = DeflateDecoder::new(BufReader::new(data));
        bincode::deserialize_from(&mut decoder)
            .map_err(|err| format!("Error deserializing decompressed binary into compressed message: {:?}", err))
    }
}

pub trait ServerLogic<T: Tys, CM: Message>: Sync {
    /// Called every tick; the resulting messages are broadcast to every connected client.
    fn tick(&mut self, seq: u32, universe: &mut T::U) -> Option<Vec<T::ServerMessage>>;
    /// Called for every message received from a client; the resulting messages are broadcast to the
    /// client that sent the message.
    fn handle_client_message(
        &mut self,
        seq: Arc<AtomicU32>,
        &CM
    ) -> Box<Future<Item=Option<T::ServerMessage>, Error=!>>;
}

impl<'d, T> CompressedMessage for T where T:Debug + Eq + CompressedMessage, for<'de> Self: Deserialize<'de> {}
