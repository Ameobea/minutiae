//! Defines the logic for the websocket server.  This server is responsible for managinc the connections to all of the
//! clients and passing messages to them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::intrinsics::type_name;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use futures::{Future, Sink, Stream};
use futures::sync::mpsc::{unbounded, UnboundedSender};
use futures::sync::oneshot::{channel as oneshot_channel, Receiver as OneshotReceiver};
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

struct Counter(Id);

impl Counter {
	fn new() -> Self {
		Counter(0)
	}
}

impl Iterator for Counter {
	type Item = Id;

	fn next(&mut self) -> Option<Id> {
		if self.0 != Id::max_value() {
			self.0 += 1;
			Some(self.0)
		} else {
			None
		}
	}
}

type WebsocketClientSink = SplitSink<Framed<TcpStream, MessageCodec<OwnedWsMessage>>>;

#[derive(Clone)]
pub struct Server<
    T: Tys,
    L: ServerLogic<T>,
> {
    seq: Arc<AtomicU32>,
    logic: Arc<Mutex<L>>,
    connection_map: Arc<RwLock<HashMap<Id, UnboundedSender<OwnedWsMessage>>>>,
    __phantom_t: PhantomData<T>,
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
    L: ServerLogic<T> + Send + 'static,
> (
    msg: OwnedWsMessage,
    seq: Arc<AtomicU32>,
    logic: Arc<Mutex<L>>
) -> Box<Future<Item=Option<OwnedWsMessage>, Error=WebSocketError>> where
    T::ServerMessage: 'static,
    T::ClientMessage: Debug,
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
            let client_msg: T::ClientMessage = match T::ClientMessage::bin_deserialize(&msg_content) {
                Ok(m) => m,
                Err(err) => {
                    println!("Error deserializing `ClientMessage` from binary data sent from user: {:?}", err);
                    return box ok(None);
                }
            };
            println!("Received message from client: {:?}", client_msg);

            // Handle the received message with the provided server logic
            box logic
                .lock()
                .unwrap()
                .handle_client_message(seq.load(Ordering::Relaxed), client_msg)
                .and_then(|opt| Ok(opt.map(|server_msg| {
                    println!(
                        "Serializing `{}` into binary to send to client...",
                        unsafe { type_name::<T::ServerMessage>() }
                    );
                    OwnedWsMessage::Binary(server_msg
                        .bin_serialize()
                        .expect("Error while serializing `ServerMessage` into binary!")
                )})))
                .map_err(|_| unreachable!())
        }
        OwnedWsMessage::Close(_) => {
            println!("Received close message from client");
            // TODO: Handle??
            box ok(None)
        },
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
    L: ServerLogic<T> + Send + 'static,
>(
    ws_host: &'static str,
    seq: Arc<AtomicU32>,
    logic: Arc<Mutex<L>>
) -> OneshotReceiver<Arc<RwLock<HashMap<Id, UnboundedSender<OwnedWsMessage>>>>> where
    T::ServerMessage: 'static,
    T::ClientMessage: Debug,
{
    let (oneshot_tx, oneshot_rx) = oneshot_channel();

    thread::spawn(move || {
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

        // send the connection map back to the main thread
        oneshot_tx.send(connection_map).unwrap();

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
                let connection_handler_future = upgrade
                    // .use_protocol("rust-websocket") // TODO: Check if this is a problem
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

                spawn_future(connection_handler_future, "Client Status", &handle_clone);
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
                                        return Err(format!(
                                            "No entry in connection map for client {}; assuming they disconnected.",
                                            client_id
                                        ));
                                    }
                                };

                                client_tx.unbounded_send(msg).expect("Unable to send message through server message channel!");
                                Ok(())
                            },
                            None => {
                                println!("Server response handler returned `None`.");
                                Ok(())
                            },
                        })
                        .map_err(|err| {
                            println!("Error while handling client message: {}", err);
                        })
                });

                Ok(())
            })
        });

        // spawn the server's client message handler future on the event loop as well
        handle.spawn(client_stream_handler);

        core.run(connection_handler).unwrap();
    });

    oneshot_rx
}

impl<
    T: Tys,
    L: ServerLogic<T> + Send + 'static,
> Server<T, L> where
    T::ServerMessage: 'static,
    T::ClientMessage: Debug,
{
    pub fn new(
        ws_host: &'static str,
        logic: L,
        seq: Arc<AtomicU32>
    ) -> Box<Self> {
        let logic = Arc::new(Mutex::new(logic));
        let connection_map = get_ws_server_future(
            ws_host,
            Arc::clone(&seq),
            Arc::clone(&logic)
        ).wait().unwrap();

        box Server {
            seq,
            logic,
            connection_map,
            __phantom_t: PhantomData,
        }
    }
}

impl<
    T: Tys,
    L: ServerLogic<T>,
> Server<T, L> {
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
                let _ = client_tx.unbounded_send(binary_msg);
            }
        }
    }
}

impl<
    T: Tys,
    L: ServerLogic<T>,
    N: Engine<T::C, T::E, T::M, T::CA, T::EA, T::U>,
> Middleware<T::C, T::E, T::M, T::CA, T::EA, T::U, N> for Box<Server<T, L>> {
    fn after_render(&mut self, universe: &mut T::U) {
        let mut logic_inner = self.logic.lock().unwrap();
        if let Some(msgs) = logic_inner.tick(self.get_seq(), universe) {
            // Broadcast to all connected clients
            self.broadcast_messages(&msgs);
        }
        self.seq.fetch_add(1, Ordering::Relaxed);
    }
}
