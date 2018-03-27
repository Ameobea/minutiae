//! Defines the logic for the websocket server.  This server is responsible for managinc the connections to all of the
//! clients and passing messages to them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};

use futures::{Future, Sink, Stream};
use futures::stream::{iter_ok, SplitSink};
use futures::sync::mpsc;
use futures::future::ok;
use futures_cpupool::CpuPool;
use tokio_core::reactor::{Handle, Core};
use websocket::message::OwnedMessage as OwnedWsMessage;
use websocket::result::WebSocketError;
use websocket::async::Server as AsyncWsServer;
use websocket::async::{MessageCodec, TcpStream};
use websocket::client::async::Framed;

use engine::Engine;
use driver::middleware::Middleware;

use super::*;

type Id = u32;

struct Counter {
	count: Id,
}
impl Counter {
	fn new() -> Self {
		Counter { count: 0 }
	}
}


impl Iterator for Counter {
	type Item = Id;

	fn next(&mut self) -> Option<Id> {
		if self.count != Id::max_value() {
			self.count += 1;
			Some(self.count)
		} else {
			None
		}
	}
}

type WebsocketClientSink = SplitSink<Framed<TcpStream, MessageCodec<OwnedWsMessage>>>;

#[derive(Clone)]
pub struct Server<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> {
    seq: Arc<AtomicU32>,
    logic: Arc<L>,
    connection_map: Arc<RwLock<HashMap<Id, WebsocketClientSink>>>,
    event_loop_handle: Handle,
    __phantom_T: PhantomData<T>,
    __phantom_CM: PhantomData<CM>,
}

fn spawn_future<
    F: Future<Item = I, Error = E> + 'static,
    I,
    E: Debug
>(f: F, desc: &'static str, handle: &Handle) {
    let mapped_future = f
        .map_err(move |e| println!("{}: '{:?}'", desc, e))
	    .map(move |_| println!("{}: Finished.", desc));
	handle.spawn(mapped_future);
}

fn handle_client_message<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
> (
    msg: OwnedWsMessage,
    seq: Arc<AtomicU32>,
    logic: Arc<L>
) -> Box<Future<Item=Option<OwnedWsMessage>, Error=WebSocketError>> where
    T::ServerMessage: 'static
{
    println!("Message from Client: {:?}", msg);
    match msg {
        OwnedWsMessage::Ping(p) => box ok(Some(OwnedWsMessage::Pong(p))),
        OwnedWsMessage::Pong(_) => box ok(None),
        OwnedWsMessage::Text(_) => {
            println!("Text message received from client; ignoring.");
            box ok(None)
        },
        OwnedWsMessage::Binary(msg_content) => {
            // Deserialize into the appropriate `ClientMessage`
            let client_msg: CM = match CM::bin_deserialize(&msg_content) {
                Ok(m) => m,
                Err(err) => {
                    println!("Error deserializing `ClientMessage` from binary data sent from user: {:?}", err);
                    return box ok(None);
                }
            };

            // Handle the received message with the provided server logic
            box logic.handle_client_message(seq, &client_msg)
                .and_then(|opt| Ok(opt.map(|server_msg| {
                    OwnedWsMessage::Binary(server_msg
                        .bin_serialize()
                        .expect("Error while serializing `ServerMessage` into binary!")
                )})))
                .map_err(|_| unreachable!())
        }
        _ => unreachable!(),
    }
}

fn get_ws_server_future<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
>(
    seq: Arc<AtomicU32>,
    logic: Arc<L>
) -> (
    Handle,
    Arc<RwLock<HashMap<Id, WebsocketClientSink>>>,
) where
    T::ServerMessage: 'static,
{
    // Set up Tokio stuff
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let handle_clone = handle.clone();
    let pool = Rc::new(CpuPool::new_num_cpus());
    let remote = core.remote();

    // Connection map that matches connection IDs with the futures sink that
    // can be used to send messages to them individually
	let connection_map = Arc::new(RwLock::new(HashMap::new()));
    let connection_map_clone = Arc::clone(&connection_map);
    // Channel which can be used to send the client sink out
	let (receive_channel_out, receive_channel_in) = mpsc::unbounded();
    let conn_id = Rc::new(RefCell::new(Counter::new()));

    // Initialize the websocket server
    let server = AsyncWsServer::bind("0.0.0.0:7037", &handle).unwrap();

    let connection_handler = server.incoming()
        // We don't want to save the stream if it drops
        .map_err(|_| ())
        .for_each(move |(upgrade, addr)| {
            let connection_map = connection_map_clone.clone();
            println!("Got a connection from: {}", addr);
            let channel = receive_channel_out.clone();
            let handle = handle_clone.clone();
            let handle_clone = handle.clone();
            let conn_id_clone = conn_id.clone();

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket") // TODO: Check if this is a problem
                .accept()
                .and_then(move |(framed, _)| {
                    let (sink, client_message_stream) = framed.split();
                    // Create an ID for this client using our counter
                    let client_id = conn_id_clone
                        .borrow_mut()
                        .next()
                        .expect("maximum amount of ids reached");

                    // Transfer out the stream that contains messages from the client and
                    // handle it on the CPU Pool
                    let f = channel.send((client_id, client_message_stream));
                    spawn_future(f, "Sent stream to connection pool", &handle);
                    // Insert the new connection into the connection map
                    connection_map
                        .write()
                        .unwrap()
                        .insert(client_id, sink);

                    Ok(())
                });

            spawn_future(f, "Client Status", &handle_clone);
            Ok(())
        });

    // Handle receiving messages from a client
	let remote_inner = remote.clone();
	pool.spawn_fn(move || {
		receive_channel_in.for_each(move |(id, client_message_stream)| {
			remote_inner.spawn(move |_| {
                client_message_stream
                    .and_then(move |msg| {
                        handle_client_message(msg, Arc::clone(&seq), Arc::clone(&logic))
                    })
                    .for_each(move |server_message_opt| match server_message_opt {
                        Some(msg) => {
                            // We have a message that needs to get sent back to the client
                            Ok(()) // TODO
                        },
                        None => Ok(()),
                    })
                    .map_err(|_| ())
            });
			Ok(())
		})
	});

    // Spawn the server's logic future on the event loop
    core.run(connection_handler).unwrap();

    (handle, connection_map)
}

impl<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
> Server<T, CM, L> where
    Self: Clone,
    T::ServerMessage: 'static,
{
    pub fn new(
        ws_host: &'static str,
        logic: L,
        seq: Arc<AtomicU32>
    ) -> Box<Self> {
        let seq = Arc::new(AtomicU32::new(0));
        let logic = Arc::new(logic);
        let (event_loop_handle, connection_map) = get_ws_server_future(
            Arc::clone(&seq),
            Arc::clone(&logic)
        );

        box Server {
            seq,
            logic,
            connection_map,
            event_loop_handle,
            __phantom_CM: PhantomData,
            __phantom_T: PhantomData,
        }
    }
}

impl<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> Server<T, CM, L> {
    pub fn get_seq(&self) -> u32 {
        self.seq.load(Ordering::Relaxed)
    }

    fn broadcast_messages(&self, server_msgs: &[T::ServerMessage]) {
        let binary_messages: Vec<OwnedWsMessage> = server_msgs
            .into_iter()
            .map(|sm| OwnedWsMessage::Binary(sm.bin_serialize().unwrap()))
            .collect();

        for (client_id, server_msg_sink) in &*self.connection_map.read().unwrap() {
            let binary_messages_stream = iter_ok::<_, ()>(binary_messages.clone());
            let response_msg_future = binary_messages_stream
                .map_err(|_| -> WebSocketError { unreachable!() })
                .forward(*server_msg_sink)
                .map_err(|ws_err| println!("Error while broadcasting message to client: {:?}", ws_err))
                .map(|res| ());

            // Spawn the response future on the event loop
            self.event_loop_handle.spawn(response_msg_future);
        }
    }
}

impl<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
    N: Engine<T::C, T::E, T::M, T::CA, T::EA, T::U>,
> Middleware<T::C, T::E, T::M, T::CA, T::EA, T::U, N> for Box<Server<T, CM, L>> {
    fn after_render(&mut self, universe: &mut T::U) {
        if let Some(msgs) = self.logic.tick(universe) {
            // Broadcast to all connected clients
            self.broadcast_messages(&msgs);
        }
        self.seq.fetch_add(1, Ordering::Relaxed);
    }
}
