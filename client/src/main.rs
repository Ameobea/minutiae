//! Minutiae simulation client.  See README.md for more information.

extern crate uuid;
extern crate minutiae;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_int, c_void};
use std::slice::from_raw_parts;

use uuid::Uuid;

use minutiae::server::*;

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

mod thin;
mod hybrid;
mod fat;

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

/// The base client that implements logic shared by all varieties of client.  Is used to synchronize and order
/// messages with the client as well as keep track of the client's (possibly abstracted) view of the universe.
pub struct ClientState<S, SM: ServerMessage<S>> {
    pub message_buffer: Vec<SM>,
    pub last_seq: u32,
    pub uuid: Uuid,
    pub pending_shapshot: bool,
    __phantom_s: PhantomData<S>,
}

impl<S, SM: ServerMessage<S>> ClientState<S, SM> {
    pub fn new() -> Self {
        ClientState {
            message_buffer: Vec::new(),
            last_seq: 0,
            uuid: Uuid::new_v4(),
            pending_shapshot: true,
            __phantom_s: PhantomData,
        }
    }
}

pub trait Client<S, SM: ServerMessage<S>> {
    fn handle_message(&mut self, message: SM);
    fn apply_snap(&mut self, snap: S);
    fn get_pixbuf_ptr(&self) -> *const u8;
    fn get_state(&mut self) -> &mut ClientState<S, SM>;
}

type ActiveClient = thin::ThinClient;
#[cfg(feature="hybrid")]
type ActiveClient = hybrid::HybridClientt;
#[cfg(feature="fat")]
type ActiveClient = fat::FatClient;

type ActiveClientMessage = ThinClientMessage;
#[cfg(feature="hybrid")]
type ActiveClientMessage = HybridClienttMessage;
#[cfg(feature="fat")]
type ActiveClientMessage = FatClientMessage;

type ActiveServerMessage = ThinServerMessage;
#[cfg(feature="hybrid")]
type ActiveServerMessage = HybridServerMessage;
#[cfg(feature="fat")]
type ActiveServerMessage = FatServerMessage;

/// Creates a client allocated on the heap and returns a pointer to it.
#[no_mangle]
pub extern "C" fn create_client(universe_size: c_int) -> *mut c_void {
    let client = thin::ThinClient::new(universe_size as usize);
    #[cfg(feature="hybrid")]
    let client = hybrid::HybridClient::new(universe_size as usize);
    #[cfg(feature="fat")]
    let client = fat::FatClient::new(universe_size as usize);
    Box::into_raw(Box::new(client)) as *mut c_void
}

/// Given a client, returns a pointer to its inner pixel data buffer.
#[no_mangle]
pub unsafe extern "C" fn get_buffer_ptr(client: *const ActiveClient) -> *const u8 {
    (*client).get_pixbuf_ptr() as *const u8
}

#[no_mangle]
pub extern "C" fn process_message(client: *mut ActiveClient, message_ptr: *const u8, message_len: c_int) {
    debug(&format!("Received message of size {} from server.", message_len));
    // debug(&format!("Processing message of length {} bytes...", message_len));
    let mut client: &mut ActiveClient = unsafe { &mut *client };
    // construct a slice from the raw data
    let slice: &[u8] = unsafe { from_raw_parts(message_ptr, message_len as usize) };
    // decompress and deserialize the message buffer into a `ThinServerMessage`
    let message: ActiveServerMessage = match ActiveServerMessage::deserialize(slice) {
        Ok(msg) => msg,
        Err(err) => {
            println!("Error while deserializing `ThinServerMessage`: {:?}", err);
            return;
        },
    };

    handle_message(client, message);

    // We don't need to free the message buffer on the Rust side; that will be handled from the JS
    // The same goes for drawing the updated universe to the canas.
}

// TODO: Actually use sequence numbers in a somewhat intelligent manner
// TODO: Wait until the response from the snapshot request before applying diffs
fn handle_message(client: &mut ActiveClient, message: ActiveServerMessage) {
    let seq = message.get_seq();

    if client.get_state().pending_shapshot {
        // we have to wait until we receive the snapshot before we can start applying diffs, so
        // queue up all received diffs until we get the snapshot
        match message.get_snapshot() {
            Ok(snap) => {
                debug(&format!("Received snapshot message with seq {}", seq));
                client.get_state().pending_shapshot = false;
                client.get_state().last_seq = seq;
                client.apply_snap(snap);
                // swap the buffer out of the state so we can mutably borrow the client
                let mut messages = mem::replace(&mut client.get_state().message_buffer, Vec::new());
                // sort all pending messages, discard any from before the snapshot was received, and apply the rest
                messages.sort();
                for queued_msg in messages {
                    if queued_msg.get_seq() > seq {
                        handle_message(client, queued_msg);
                    }
                }
            },
            Err(msg) => client.get_state().message_buffer.push(msg),
        }

        return;
    }

    if seq == client.get_state().last_seq + 1 || client.get_state().last_seq == 0 {
        client.handle_message(message);

        client.get_state().last_seq += 1;

        // if we have buffered messages to handle, apply them now.
        for msg in mem::replace(&mut client.get_state().message_buffer, Vec::new()) {
            client.handle_message(msg);
        }
    } else if seq > client.get_state().last_seq + 1 {
        debug(&format!("Received message with sequence number greater than what we expected: {}", seq));

        // store the message in the client's message buffer and wait until we receive the missing ones
        client.get_state().message_buffer.push(message);

        // if it's been a long time since we've missed the message, give up and request a new snapshot to refresh our state.
        if seq > (client.get_state().last_seq + 60) {
            debug(&format!("Missed message with sequence number {}; sending snapshot request...", seq));
            request_snapshot_inner(client);
            client.get_state().pending_shapshot = true;
        }
    } else if seq == client.get_state().last_seq {
        debug(&format!("Received duplicate message with sequence number {}: {:?}", seq, message));
    }
}

#[no_mangle]
/// Sends a message to the server requesting a snapshot of the current universe
pub unsafe extern "C" fn request_snapshot(client: *mut ActiveClient) {
    request_snapshot_inner(&mut *client);
}

fn request_snapshot_inner(client: &mut ActiveClient) {
    let msg = ActiveClientMessage::create_snapshot_request((*client).get_state().uuid).serialize().unwrap();
    debug("Sending message requesting snapshot from the server...");
    unsafe { send_client_message(msg.as_ptr(), msg.len() as i32) };
}

pub fn main() {
    // create the websocket connection and start handling server messages
    println!("Initializing WS connection from the Rust side...");
    unsafe { init_ws() };
}
