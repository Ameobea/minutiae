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
use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::stream::SplitSink;
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
    connection_map: Arc<RwLock<HashMap<Id, UnboundedSender<OwnedWsMessage>>>>,
    __phantom_t: PhantomData<T>,
    __phantom_cm: PhantomData<CM>,
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

/// Generates a stream that feeds messages into the client sink provided to it.
fn create_client_sink_handler(
    sink: WebsocketClientSink
) -> (impl Future<Item=(), Error=()>, UnboundedSender<OwnedWsMessage>) {
    let (tx, rx) = unbounded();

    // Forward all the messages from the
    let driver_future = rx
        .map_err(|()| -> WebSocketError { unreachable!() })
        .forward(sink)
        .map(|_| println!("WARN: Client sink fully flushed from tx")) // This should never actually reach here
        .map_err(|ws_err| {
            println!("Error while sending items through WebSocket to client: {:?}", ws_err)
        });

    (driver_future, tx)
}

fn get_ws_server_future<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
>(
    ws_host: &str,
    seq: Arc<AtomicU32>,
    logic: Arc<L>
) -> Arc<RwLock<HashMap<Id, UnboundedSender<OwnedWsMessage>>>> where
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
    let connection_map_clone2 = Arc::clone(&connection_map);
    // Channel which can be used to send the client sink out
	let (receive_channel_out, receive_channel_in) = mpsc::unbounded();
    let conn_id_generator = Rc::new(RefCell::new(Counter::new()));

    // Initialize the websocket server
    let server = AsyncWsServer::bind(ws_host, &handle).unwrap();

    let connection_handler = server.incoming()
        // We don't want to save the stream if it drops
        .map_err(|_| ())
        .for_each(move |(upgrade, addr)| {
            let connection_map = connection_map_clone.clone();
            println!("Got a connection from: {}", addr);
            let channel = receive_channel_out.clone();
            let handle = handle_clone.clone();
            let handle_clone = handle.clone();
            let conn_id_generator_clone = conn_id_generator.clone();

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket") // TODO: Check if this is a problem
                .accept()
                .and_then(move |(framed, _)| {
                    let (sink, client_message_stream) = framed.split();
                    // Create an ID for this client using our counter
                    let client_id = conn_id_generator_clone
                        .borrow_mut()
                        .next()
                        .expect("maximum amount of ids reached");

                    // Transfer out the stream that contains messages from the client and
                    // handle it on the CPU Pool
                    let f = channel.send((client_id, client_message_stream));
                    spawn_future(f, "Sent stream to connection pool", &handle);

                    // Create a stream that wraps around the sink and handles converting and
                    // sending `ServerMessage`s through it
                    let (handler_future, tx) = create_client_sink_handler(sink);
                    spawn_future(handler_future, "Client message channel mapper", &handle);

                    // Insert the new connection into the connection map
                    connection_map
                        .write()
                        .unwrap()
                        .insert(client_id, tx);

                    Ok(())
                });

            spawn_future(f, "Client Status", &handle_clone);
            Ok(())
        });

	let client_stream_handler = pool.spawn_fn(move || {
        let remote_clone = remote.clone();
        let seq = seq;
        let connection_map = Arc::clone(&connection_map_clone2);

		receive_channel_in.for_each(move |(client_id, client_message_stream)| {
            let seq_clone = Arc::clone(&seq);
            let remote_inner = remote_clone.clone();
            let logic_clone = Arc::clone(&logic);
            let connection_map = Arc::clone(&connection_map);

			remote_inner.spawn(move |_| {
                let seq_clone_clone = Arc::clone(&seq_clone);
                let logic_clone_clone = Arc::clone(&logic_clone);

                client_message_stream
                    .and_then(move |msg| {
                        handle_client_message(msg, Arc::clone(&seq_clone_clone), Arc::clone(&logic_clone_clone))
                    })
                    .map_err(|ws_err| format!("{:?}", ws_err))
                    .for_each(move |server_msg_opt| match server_msg_opt {
                        Some(msg) => {
                            // We have a message that needs to get sent back to the client
                            // Get the client's sink out of the connection map
                            let mut connection_map_inner = connection_map
                                .read()
                                .expect("Unable to lock connect_map for reading in message handler!");
                            let client_tx = match connection_map_inner.get(&client_id) {
                                Some(tx) => tx,
                                None => {
                                    return Err(format!("No entry in connection map for client {}; assuming they disconnected.", client_id));
                                }
                            };

                            client_tx.unbounded_send(msg).expect("Unable to send message through server message channel!");
                            Ok(())
                        },
                        None => Ok(()),
                    })
                    .map_err(|err| {
                        println!("Error while handling client message: {}", err);
                    })
            });

			Ok(())
		})
	});

    // Spawn the server's connection handler future on the event loop
    core.run(connection_handler).unwrap();
    // spawn the server's client message handler future on the event loop as well
    core.run(client_stream_handler).unwrap();

    connection_map
}

impl<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
> Server<T, CM, L> where T::ServerMessage: 'static {
    pub fn new(
        ws_host: &'static str,
        logic: L,
        seq: Arc<AtomicU32>
    ) -> Box<Self> {
        let logic = Arc::new(logic);
        let connection_map = get_ws_server_future(
            ws_host,
            Arc::clone(&seq),
            Arc::clone(&logic)
        );

        box Server {
            seq,
            logic,
            connection_map,
            __phantom_cm: PhantomData,
            __phantom_t: PhantomData,
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

        for (_client_id, client_tx) in &*self.connection_map.write().unwrap() {
            for binary_msg in binary_messages.clone() {
                client_tx.unbounded_send(binary_msg)
                    .expect("Unable to broadcast message to client");
            }
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
