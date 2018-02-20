//! Sets up code for communicating changes in universe state with remote clients.

use std::sync::{Arc, RwLock};
use std::fmt::Debug;
use std::io::BufReader;
use std::cmp::{PartialOrd, Ord};

use bincode::{self, serialize_into, serialized_size};
use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use prelude::*;

#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use self::server::*;

#[cfg(feature = "server")]
mod thin;
#[cfg(feature = "server")]
pub use self::thin::*;
#[cfg(feature = "server")]
mod hybrid;
#[cfg(feature = "server")]
pub use self::hybrid::*;
#[cfg(feature = "server")]
mod fat;
#[cfg(feature = "server")]
pub use self::fat::*;

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
        bincode::serialize(self).map_err(|_| String::from("Unable to serialize message."))
    }

    fn bin_deserialize(data: &[u8]) -> Result<Self, String> {
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

impl<'d, T> CompressedMessage for T where T:Debug + Eq + CompressedMessage, for<'de> Self: Deserialize<'de> {}

