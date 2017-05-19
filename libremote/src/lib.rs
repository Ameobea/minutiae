//! Minutae libremote.  See README.md for additional information.

extern crate bincode;
extern crate flate2;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::io::Read;

use bincode::{serialize, deserialize, Infinite};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use serde::{Serialize, Deserialize};

/// All messages that are passed between the server and clients are of this form.  Each message is accompanied by a sequence
/// number that is used to ensure that they're applied in order.  There will never be a case in which sequence numbers are
/// skipped; if a client misses a message or receives an out-of-order message, it should be stored until the missing one is
/// received or a message should be sent requesting a re-broadcast.
#[derive(Serialize, Deserialize)]
pub struct ServerMessage {
    pub seq: u32,
    pub contents: ServerMessageContents,
}

impl ServerMessage {
    /// Encodes the message in binary format, compressing it in the process.
    pub fn serialize(self) -> Result<Vec<u8>, String> {
        let binary = serialize(&self, Infinite).map_err(|_| String::from("Error while serializing `ServerMessage`."))?;
        let encoder = ZlibEncoder::new(binary, Compression::Default);
        encoder.finish().map_err(|err| format!("Error while reading bytes out of compressor: {:?}", err))
    }

    /// Decodes and decompresses a binary-encoded message.
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut decoder = ZlibDecoder::new(data);
        let mut binary: Vec<u8> = Vec::with_capacity(data.len());
        decoder.read_to_end(&mut binary)
            .map_err(|err| format!("Unable to decompress binary `ServerMessage`: {:?}", err))?;
        deserialize(&binary[..])
            .map_err(|err| format!("Error deserializing decompressed binary into `ServerMessage`: {:?}", err))?
    }
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessageContents {
    Diff(Vec<Diff>),
    Snapshot(Vec<u8>),
}

/// Encodes the difference between two different steps of a simulation.  Currently simply contains a universe index and
/// and the object that is visible there.
#[derive(Serialize, Deserialize)]
pub struct Diff {
    pub universe_index: usize,
    pub new_object: u8,
}
