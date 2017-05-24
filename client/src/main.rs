//! Minutiae simulation client.  See README.md for more information.

extern crate uuid;
extern crate minutiae_libremote;

use std::ffi::CString;
use std::mem;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::write;
use std::slice::from_raw_parts;

use uuid::Uuid;
use minutiae_libremote::{ClientMessageContent, Color, Diff, ServerMessage, ServerMessageContents};

extern {
    /// Used to initialize the websocket connection and start receiving+processing messages from the server
    pub fn init_ws();
    /// Used to send a binary-encoded `ClientMessage` over the websocket to the server.
    pub fn send_client_message(ptr: *const u8, len: c_int);
    /// Direct line to `console.log` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_debug(msg: *const c_char);
    /// Direct line to `console.error` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_error(msg: *const c_char);
}

/// Wrapper around the JS debug function that accepts a Rust `&str`.
fn debug(msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { js_debug(c_str.as_ptr()) };
}

/// Wrapper around the JS error function that accepts a Rust `&str`.
fn error(msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { js_error(c_str.as_ptr()) };
}

/// Holds the state of the universe as viewed the client.  Only holds a shallow view as calculated by the server.
pub struct Client {
    pub universe: Vec<[u8; 4]>,
    pub message_buffer: Vec<ServerMessage>,
    pub last_seq: u32,
    pub uuid: Uuid,
    pub received_snapshot: bool,
}

impl Client {
    pub fn new(universe_size: usize) -> Self {
        Client {
            universe: vec![[0u8, 0u8, 0u8, 255u8]; universe_size * universe_size * 4],
            message_buffer: Vec::new(),
            last_seq: 0,
            uuid: Uuid::new_v4(),
            received_snapshot: false,
        }
    }

    pub fn apply_diff(&mut self, diff: Diff) {
        debug_assert!(diff.universe_index < self.universe.len());
        unsafe {
            let ptr = self.universe.get_unchecked_mut(diff.universe_index).as_mut_ptr() as *mut [u8; 3];
            write(ptr, diff.color.0);
        };
    }

    pub fn apply_snap(&mut self, snap: Vec<Color>) {
        debug("Received snapshot from server... attempting to apply it.");
        debug_assert_eq!(self.universe.len(), snap.len() / 4);
        for (i, color) in snap.iter().enumerate() {
            self.apply_diff(Diff {universe_index: i, color: *color});
        }
    }
}

/// Creates a client allocated on the heap and returns a pointer to it.
#[no_mangle]
pub extern "C" fn create_client(universe_size: c_int) -> *mut c_void {
    let boxed_client = Box::new(Client::new(universe_size as usize));
    Box::into_raw(boxed_client) as *mut c_void
}

/// Given a client, returns a pointer to its inner pixel data buffer.
#[no_mangle]
pub unsafe extern "C" fn get_buffer_ptr(client: *const Client) -> *const u8 {
    (*(*client).universe).as_ptr() as *const u8
}

#[no_mangle]
pub extern "C" fn process_message(client: *mut Client, message_ptr: *const u8, message_len: c_int) {
    debug(&format!("Received message of size {} from server.", message_len));
    // debug(&format!("Processing message of length {} bytes...", message_len));
    let mut client: &mut Client = unsafe { &mut *client };
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

    handle_message(&mut client, message);

    // We don't need to free the message buffer on the Rust side; that will be handled in the JS
    // The same goes for drawing the updated universe to the canas.
}

// TODO: Actually use sequence numbers in a somewhat intelligent manner
// TODO: Wait until the response from the snapshot request before applying diffs
fn handle_message(client: &mut Client, message: ServerMessage) {
    if !client.received_snapshot {
        // we have to wait until we receive the snapshot before we can start applying diffs, so
        // queue up all received diffs until we get the snapshot
        match message.contents {
            ServerMessageContents::Snapshot(snap) => {
                debug(&format!("Received snapshot message with seq {}", message.seq));
                client.received_snapshot = true;
                client.last_seq = message.seq;
                client.apply_snap(snap);
                // swap the buffer out of the client so we can mutably borrow the client
                let mut messages = mem::replace(&mut client.message_buffer, Vec::new());
                // sort all pending messages, discard any from before the snapshot was received, and apply the rest
                messages.sort();
                for msg in messages {
                    if msg.seq > message.seq {
                        handle_message(client, msg);
                    }
                }
            },
            ServerMessageContents::Diff(_) => client.message_buffer.push(message),
        }
        return;
    }

    if message.seq == client.last_seq + 1 || client.last_seq == 0 {
        match message.contents {
            ServerMessageContents::Diff(diffs) => {
                // apply all diffs contained in the message
                for diff in diffs {
                    client.apply_diff(diff);
                }
            },
            ServerMessageContents::Snapshot(snap) => client.apply_snap(snap),
        }

        client.last_seq += 1;

        // if we have buffered messages to handle, apply them now.
        let diffs_list: Vec<Vec<Diff>> = client.message_buffer.drain(..).map(|message| -> Vec<Diff> {
            match message.contents {
                ServerMessageContents::Diff(diffs) => diffs,
                _ => Vec::new(),
            }
        }).collect();

        for diffs in diffs_list {
            for diff in diffs {
                client.apply_diff(diff);
            }
        }
    } else if message.seq > client.last_seq + 1 {
        debug(&format!("Received message with sequence number greater than what we expected: {}", message.seq));
        // store the message in the client's message buffer and wait until we receive the missing ones
        client.message_buffer.push(message);
        client.message_buffer.sort();

        // if it's been a while since we lost the message, ask to retransmit it and any others we're missing
        let mut last_seen_seq = client.last_seq;
        for message in &client.message_buffer {
            // send retransmission  requests for the missing messages
            for missing_seq in (last_seen_seq + 1)..message.seq {
                // let client_msg_bin: Vec<u8> = ClientMessage::Retransmit(missing_seq)
                //     .serialize()
                //     .expect("Unable to serialize `ClientMessage` while requesting message retransmission!");
                // unsafe { send_client_message((&client_msg_bin).as_ptr(), client_msg_bin.len() as c_int) }
                // heap-allocated serialized message will be freed here.
            }

            last_seen_seq = message.seq;
        }
    } else if message.seq == client.last_seq {
        debug(&format!("Received duplicate message with sequence number {}", message.seq));
        // TODO
    }
}

#[no_mangle]
/// Sends a message to the server requesting a snapshot of the current universe
pub unsafe extern "C" fn request_snapshot(client: *const Client) {
    let msg = ClientMessageContent::SendSnapshot.serialize((*client).uuid).unwrap();
    debug("Sending message requesting snapshot from the server...");
    send_client_message(msg.as_ptr(), msg.len() as i32);
}

pub fn main() {
    // create the websocket connection and start handling server messages
    println!("Initializing WS connection from the Rust side...");
    unsafe { init_ws() };
}
