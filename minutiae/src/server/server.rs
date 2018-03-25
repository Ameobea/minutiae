//! Defines the logic for the websocket server.  This server is responsible for managinc the connections to all of the
//! clients and passing messages to them.

use std::thread;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use ws::{self, WebSocket, Handler};

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use engine::Engine;
use driver::middleware::Middleware;

use super::*;

#[derive(Clone)]
pub struct Server<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> {
    pub logic: L,
    // sender that can be used to broadcast a message to all connected clients
    pub ws_broadcaster: ws::Sender,
    pub seq: Arc<AtomicU32>,
    __phantom_T: PhantomData<T>,
    __phantom_CM: PhantomData<CM>,
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
        let logic_clone = logic.clone();

        Box::new(Server {
            logic,
            ws_broadcaster: init_ws_server(ws_host, logic_clone, seq.clone()),
            seq,
            __phantom_T: PhantomData,
            __phantom_CM: PhantomData,
        })
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

struct WsServerHandler<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
>  {
    seq: Arc<AtomicU32>,
    out: ws::Sender,
    logic: L,
    __phantom_T: PhantomData<T>,
    __phantom_CM: PhantomData<CM>,
}

impl<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> WsServerHandler<T, CM, L> {
    pub fn new(
        seq: Arc<AtomicU32>,
        out: ws::Sender,
        logic: L,
    ) -> Self {
        WsServerHandler {
            seq,
            out,
            logic,
            __phantom_T: PhantomData,
            __phantom_CM: PhantomData,
        }
    }
}

impl<
    T: Tys,
    CM: Message,
    L: ServerLogic<T, CM> + Clone,
> Handler for WsServerHandler<T, CM, L> {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        match msg {
            ws::Message::Binary(bin) => {
                // try to convert the received message into a `ClientMessage`
                let client_msg: CM = match CM::bin_deserialize(&bin) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("Error deserializing `ClientMessage` from binary data sent from user: {:?}", err);
                        return Ok(())
                    }
                };

                match self.logic.handle_client_message(Arc::clone(&self.seq), &client_msg) {
                    Some(msgs) => {
                        // serialize and transmit the messages to the client
                        for msg in msgs {
                            let serialized: Vec<u8> = msg.bin_serialize().expect("Unable to send message to client!");
                            // TODO: Look into handling errors
                            self.out.send::<&[u8]>(serialized.as_slice().into()).unwrap();
                        }
                    },
                    None => (),
                }
            },
            ws::Message::Text(text) => println!("Someone tried to send a text message over the WebSocket: {}", text),
        }

        Ok(())
    }
}

fn init_ws_server<
    T: Tys,
    CM: Message + Send,
    L: ServerLogic<T, CM> + Clone + Send + 'static,
> (ws_host: &'static str, logic: L, seq: Arc<AtomicU32>) -> ws::Sender {
    let server = WebSocket::new(move |out: ws::Sender| {
        WsServerHandler::new(seq.clone(), out, logic.clone())
    }).expect("Unable to initialize websocket server!");

    let broadcaster = server.broadcaster();

    // start the server on a separate thread
    thread::spawn(move || {
        server.listen(ws_host).expect("Unable to initialize websocket server!");
    });

    broadcaster
}
