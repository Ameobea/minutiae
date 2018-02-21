//! Defines the logic for the websocket server.  This server is responsible for managinc the connections to all of the
//! clients and passing messages to them.

use std::{mem, ptr, thread};
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

pub trait ServerLogic<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    SM: Message,
    CM: Message,
    U: Universe<C, E, M>,
> : Sized + Clone {
    // called every tick; the resulting messages are broadcast to every connected client.
    fn tick(&mut self, universe: &mut U) -> Option<Vec<SM>>;
    // called for every message received from a client.
    fn handle_client_message(&mut self, seq: Arc<AtomicU32>, &CM) -> Option<Vec<SM>>;
}

#[derive(Clone)]
pub struct Server<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    SM: Message,
    CM: Message,
    U: Universe<C, E, M>,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U> + Clone,
> {
    pub logic: L,
    // sender that can be used to broadcast a message to all connected clients
    pub ws_broadcaster: ws::Sender,
    pub seq: Arc<AtomicU32>,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_sm: PhantomData<SM>,
    __phamtom_cm: PhantomData<CM>,
    __phantom_u: PhantomData<U>,
}

impl<
    C: CellState + Send + Clone + 'static,
    E: EntityState<C> + Send + Clone + 'static,
    M: MutEntityState + Send + Clone + 'static,
    CA: CellAction<C> + Send + Clone + 'static,
    EA: EntityAction<C, E> + Send + Clone + 'static,
    SM: Message + Send + Clone + 'static,
    CM: Message + Send + Clone + 'static,
    U: Universe<C, E, M> + Send + Clone + 'static,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U> + Send + Clone + 'static
> Server<C, E, M, CA, EA, SM, CM, U, L> where Self: Clone {
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
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_sm: PhantomData,
            __phamtom_cm: PhantomData,
            __phantom_u: PhantomData,
        })
    }
}

impl<
    C: CellState + 'static,
    E: EntityState<C> + 'static,
    M: MutEntityState + 'static,
    CA: CellAction<C> + 'static,
    EA: EntityAction<C, E> + 'static,
    SM: Message + 'static,
    CM: Message + 'static,
    U: Universe<C, E, M>,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U> + 'static
> Server<C, E, M, CA, EA, SM, CM, U, L> {
    pub fn get_seq(&self) -> u32 {
        self.seq.load(Ordering::Relaxed)
    }
}

impl<
    C: CellState + 'static,
    E: EntityState<C> + 'static,
    M: MutEntityState + 'static,
    CA: CellAction<C> + 'static,
    EA: EntityAction<C, E> + 'static,
    U: Universe<C, E, M>,
    N: Engine<C, E, M, CA, EA, U>,
    SM: Message + 'static,
    CM: Message + 'static,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U> + 'static
> Middleware<C, E, M, CA, EA, U, N> for Box<Server<C, E, M, CA, EA, SM, CM, U, L>> {
    fn after_render(&mut self, universe: &mut U) {
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
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    SM: Message,
    CM: Message,
    U: Universe<C, E, M>,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U>
>  {
    seq: Arc<AtomicU32>,
    out: ws::Sender,
    logic: L,
    __phantom_c: PhantomData<C>,
    __phantom_e: PhantomData<E>,
    __phantom_m: PhantomData<M>,
    __phantom_ca: PhantomData<CA>,
    __phantom_ea: PhantomData<EA>,
    __phantom_sm: PhantomData<SM>,
    __phamtom_cm: PhantomData<CM>,
    __phantom_u: PhantomData<U>,
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    SM: Message,
    CM: Message,
    U: Universe<C, E, M>,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U>
> WsServerHandler<C, E, M, CA, EA, SM, CM, U, L> {
    pub fn new(
        seq: Arc<AtomicU32>,
        out: ws::Sender,
        logic: L,
    ) -> Self {
        WsServerHandler {
            seq,
            out,
            logic,
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_sm: PhantomData,
            __phamtom_cm: PhantomData,
            __phantom_u: PhantomData,
        }
    }
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    SM: Message,
    CM: Message,
    U: Universe<C, E, M>,
    L: ServerLogic<C, E, M, CA, EA, CM, SM, U>
> Handler for WsServerHandler<C, E, M, CA, EA, CM, SM, U, L> {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        match msg {
            ws::Message::Binary(bin) => {
                // try to convert the received message into a `ClientMessage`
                let client_msg: SM = match SM::bin_deserialize(&bin) {
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
    C: CellState + Send + Clone + 'static,
    E: EntityState<C> + Send + Clone + 'static,
    M: MutEntityState + Send + Clone + 'static,
    CA: CellAction<C> + Send + Clone + 'static,
    EA: EntityAction<C, E> + Send + Clone + 'static,
    SM: Message + Send + Clone + 'static,
    CM: Message + Send + Clone + 'static,
    U: Universe<C, E, M> + Send + Clone + 'static,
    L: ServerLogic<C, E, M, CA, EA, SM, CM, U> + Send + Clone + 'static
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
