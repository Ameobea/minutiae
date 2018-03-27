//! Defines the logic for the websocket server.  This server is responsible for managinc the connections to all of the
//! clients and passing messages to them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use futures::{Future, Sink, Stream};
use futures::sync::mpsc;
use futures::future::ok;
use futures_cpupool::CpuPool;
use tokio_core::reactor::{Handle, Core};
use websocket::message::{Message as WsMessage, OwnedMessage as OwnedWsMessage};
use websocket::result::WebSocketError;
use websocket::server::InvalidConnection;
use websocket::async::Server as AsyncWsServer;

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
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

#[derive(Clone)]
pub struct Server<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> {
    pub seq: Arc<AtomicU32>,
    pub logic: Arc<L>,
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

fn process_message(id: u32, msg: &OwnedWsMessage) ->

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
            box (*logic).handle_client_message(seq, &client_msg)
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
>(seq: Arc<AtomicU32>, logic: Arc<L>) {
    // bind to the server
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let server = AsyncWsServer::bind("0.0.0.0:7037", &handle).unwrap();
    let pool = Rc::new(CpuPool::new_num_cpus());
    let remote = core.remote();
	let connections = Arc::new(RwLock::new(HashMap::new()));
	let (receive_channel_out, receive_channel_in) = mpsc::unbounded();
    let conn_id = Rc::new(RefCell::new(Counter::new()));
	let connections_inner = connections.clone();

    let connection_handler = server.incoming()
        // We don't want to save the stream if it drops
        .map_err(|_| ())
        .for_each(|(upgrade, addr)| {
            let connections_inner = connections_inner.clone();
            println!("Got a connection from: {}", addr);
            let channel = receive_channel_out.clone();
            let handle_inner = handle.clone();
            let conn_id = conn_id.clone();

            // accept the request to be a ws connection if it does
            let f = upgrade
                .use_protocol("rust-websocket")
                .accept()
                .and_then(move |(framed, _)| {
                    let (sink, stream) = framed.split();
                    let id = conn_id
                        .borrow_mut()
                        .next()
                        .expect("maximum amount of ids reached");
                    let f = channel.send((id, stream));
                    spawn_future(f, "Senk stream to connection pool", &handle_inner);
                    connections_inner.write().unwrap().insert(id, sink);
                    Ok(())


                });

            spawn_future(f, "Client Status", &handle);
            Ok(())
        });

    // Handle receiving messages from a client
	let remote_inner = remote.clone();
	let receive_handler = pool.spawn_fn(|| {
		receive_channel_in.for_each(move |(id, stream)| {
			remote_inner.spawn(move |_| {
                stream.for_each(move |msg| {
                    process_message(id, &msg);
                    Ok(())
                }).map_err(|_| ())
            });
			Ok(())
		})
	});

    core.run(connection_handler).unwrap();
}

impl<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
> Server<T, CM, L> where Self: Clone {
    pub fn new(
        ws_host: &'static str,
        logic: L,
        seq: Arc<AtomicU32>
    ) -> Box<Self> {
        let seq = Arc::new(AtomicU32::new(0));
        let logic = Arc::new(logic);
        let f = get_ws_server_future(Arc::clone(&seq), Arc::clone(&logic));



        box Server {
            seq,
            logic,
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
}

impl<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
    N: Engine<T::C, T::E, T::M, T::CA, T::EA, T::U>,
> Middleware<T::C, T::E, T::M, T::CA, T::EA, T::U, N> for Box<Server<T, CM, L>> {
    fn after_render(&mut self, universe: &mut T::U) {
        if let Some(msgs) = self.logic.tick(universe) {
            for msg in msgs {
                // convert the message into binary format and then send it over the websocket
                match self.ws_broadcaster.send::<&[u8]>(msg.bin_serialize().unwrap().as_slice().into()) {
                    Err(err) => println!("Error while sending message over the websocket: {:?}", err),
                    _ => (),
                }
            }
        }
        self.seq.fetch_add(1, Ordering::Relaxed);
    }
}
