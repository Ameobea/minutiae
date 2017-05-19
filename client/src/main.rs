//! Minutae simulation client.  See README.md for more information.

extern crate minutae_libremote;

use std::os::raw::{c_int, c_void};
use std::slice::from_raw_parts;
use std::ptr::write;

use minutae_libremote::{ClientMessage, Diff, ServerMessage, ServerMessageContents};

extern {
    /// Used to send a binary-encoded `ClientMessage` over the websocket to the server.
    pub fn send_client_message(ptr: *const u8, len: c_int);
}

/// Holds the state of the universe as viewed the client.  Only holds a shallow view as calculated by the server.
pub struct Client {
    pub universe: Vec<[u8; 4]>,
    pub message_buffer: Vec<ServerMessage>,
    pub last_seq: u32,
}

impl Client {
    pub fn new(universe_size: usize) -> Self {
        Client {
            universe: vec![[0u8, 0u8, 0u8, 255u8]; universe_size * universe_size * 4],
            message_buffer: Vec::new(),
            last_seq: 0,
        }
    }

    pub fn apply_diff(&mut self, diff: Diff) {
        debug_assert!(diff.universe_index < self.universe.len());
        unsafe {
            let ptr = self.universe.get_unchecked_mut(diff.universe_index).as_mut_ptr() as *mut [u8; 3];
            write(ptr, diff.color.0);
        };
    }
}

/// Creates a client allocated on the heap and returns a pointer to it.
#[no_mangle]
pub extern "C" fn create_client(universe_size: c_int) -> *mut c_void {
    let boxed_client = Box::new(Client::new(universe_size as usize));
    Box::into_raw(boxed_client) as *mut c_void
}

pub extern "C" fn process_message(client: *mut Client, message_ptr: *const u8, message_len: c_int) {
    let client: &mut Client = unsafe { &mut *client };
    // construct a slice from the raw data
    let slice: &[u8] = unsafe { from_raw_parts(message_ptr, message_len as usize) };
    // decompress and deserialize the message buffer into a `ServerMessage`
    let message: ServerMessage = match ServerMessage::deserialize(slice) {
        Ok(msg) => msg,
        Err(err) => {
            println!("Error while deserializing `ServerMessage`: {:?}", err);
            return;
        },
    };

    if message.seq == client.last_seq + 1 {
        match message.contents {
            ServerMessageContents::Diff(diffs) => {
                // apply all diffs contained in the message
                for diff in diffs {
                    client.apply_diff(diff);
                }
            },
            _ => unimplemented!(),
        }
    } else {
        // TODO: handle sequences being received out of order
    }

    // don't need to free the message buffer on the Rust side; that will be handled in the JS
}

pub fn main() {
    // intentionally left empty since all of our logic is handled from the JS side
}
