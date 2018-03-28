//! Minutiae simulation client.  See README.md for more information.

#![feature(associated_type_defaults, conservative_impl_trait, core_intrinsics, iterator_step_by, nll)]

extern crate minutiae;
extern crate serde;
extern crate uuid;

extern crate colony;

use std::ffi::CString;
use std::intrinsics::type_name;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice::from_raw_parts;

use uuid::Uuid;

use minutiae::server::*;
pub use minutiae::server::Tys;

use colony::{color_calculator, ColonyTys};

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
    pub pending_snapshot: bool,
    __phantom_s: PhantomData<S>,
}

impl<S, SM: ServerMessage<S>> ClientState<S, SM> {
    pub fn new() -> Self {
        ClientState {
            message_buffer: Vec::new(),
            last_seq: 0,
            uuid: Uuid::new_v4(),
            pending_snapshot: true,
            __phantom_s: PhantomData,
        }
    }
}

static mut CLIENT_WRAPPER: *mut Box<GenClient> = ptr::null_mut();

pub trait GenClient {
    fn get_uuid(&self) -> Uuid;

    fn get_pixbuf_ptr(&self) -> *const u8;

    fn handle_bin_message(&mut self, &[u8]);

    fn create_snapshot_request(&self) -> Vec<u8>;
}

pub trait Client<T: Tys> : GenClient {
    fn apply_snap(&mut self, snap: T::Snapshot);

    fn get_state(&mut self) -> &mut ClientState<T::Snapshot, T::ServerMessage>;

    fn handle_message(&mut self, T::ServerMessage);
}

impl<T: Tys> Client<T> {
    // TODO: Actually use sequence numbers in a somewhat intelligent manner
    // TODO: Wait until the response from the snapshot request before applying diffs
    pub fn handle_binary_message(&mut self, slice: &[u8]) {
        debug(&format!("Binary message received: {:?}", slice));
        // decompress and deserialize the message buffer into a `T::ServerMessage`
        let message: T::ServerMessage = match T::ServerMessage::bin_deserialize(slice) {
            Ok(msg) => msg,
            Err(err) => {
                debug(&format!(
                    "Error while deserializing `{}`: {:?}",
                    unsafe { type_name::<T::ServerMessage>() },
                    err
                ));
                return;
            },
        };
        let seq = message.get_seq();

        if self.get_state().pending_snapshot {
            // we have to wait until we receive the snapshot before we can start applying diffs, so
            // queue up all received diffs until we get the snapshot
            if message.is_snapshot() {
                debug(&format!("Received snapshot message with seq {}", seq));
                self.get_state().pending_snapshot = false;
                self.get_state().last_seq = seq;
                self.apply_snap(message.get_snapshot().unwrap());
                // swap the buffer out of the state so we can mutably borrow the client
                let mut messages = mem::replace(&mut self.get_state().message_buffer, Vec::new());
                // sort all pending messages, discard any from before the snapshot was received, and apply the rest
                messages.sort();

                for queued_msg in messages {
                    if queued_msg.get_seq() > seq {
                        Client::handle_message(self, queued_msg);
                    }
                }
            } else {
                self.get_state().message_buffer.push(message);
            }

            return;
        }

        if seq == self.get_state().last_seq + 1 || self.get_state().last_seq == 0 {
            debug(format!("Handling message with sequence number {}", seq));
            Client::handle_message(self, message);

            self.get_state().last_seq += 1;

            // if we have buffered messages to handle, apply them now.
            for msg in mem::replace(&mut self.get_state().message_buffer, Vec::new()) {
                Client::handle_message(self, msg);
            }
        } else if seq > self.get_state().last_seq + 1 {
            debug(&format!("Received message with sequence number greater than what we expected: {}", seq));

            // store the message in the client's message buffer and wait until we receive the missing ones
            self.get_state().message_buffer.push(message);

            // if it's been a long time since we've missed the message, give up and request a new snapshot to refresh our state.
            if seq > (self.get_state().last_seq + 60) {
                debug(&format!("Missed message with sequence number {}; sending snapshot request...", seq));
                unsafe { request_snapshot() };
                self.get_state().pending_snapshot = true;
            }
        } else if seq == self.get_state().last_seq {
            debug(&format!("Received duplicate message with sequence number {}", seq));
        }
    }
}

/// Given a client, returns a pointer to its inner pixel data buffer.
#[no_mangle]
pub unsafe extern "C" fn get_buffer_ptr() -> *const u8 {
    (**CLIENT_WRAPPER).get_pixbuf_ptr() as *const u8
}

#[no_mangle]
pub unsafe extern "C" fn process_message(message_ptr: *const u8, message_len: c_int) {
    // debug(&format!("Received message of size {} from server.", message_len));
    let client: &mut GenClient = &mut **CLIENT_WRAPPER;
    // construct a slice from the raw data
    let slice: &[u8] = from_raw_parts(message_ptr, message_len as usize);
    // Pass it to the client to deserialize and process
    client.handle_bin_message(slice);

    // We don't need to free the message buffer on the Rust side; that will be handled from the JS
    // The same goes for drawing the updated universe to the canas.
}

#[no_mangle]
/// Sends a message to the server requesting a snapshot of the current universe
pub unsafe extern "C" fn request_snapshot() {
    let msg: Vec<u8> = (**CLIENT_WRAPPER).create_snapshot_request();
    debug("Sending message requesting snapshot from the server...");
    send_client_message(msg.as_ptr(), msg.len() as i32);
}

pub fn main() {
    // Initialize the global `GenClient` with a client instance
    let client: hybrid::HybridClient<ColonyTys> = hybrid::HybridClient::new(800, color_calculator);
    unsafe { CLIENT_WRAPPER = Box::into_raw(Box::new(Box::new(client))) };
    debug(&format!("ServerMessage size: {}", ::std::mem::size_of::<<ColonyTys as Tys>::ServerMessage>()));

    // create the websocket connection and start handling server messages
    debug("Initializing WS connection from the Rust side...");
    unsafe { init_ws() };
}
