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
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>,
    SM: Message, CM: Message
> : Sized {
    // called every tick; the resulting messages are broadcast to every connected client.
    fn tick(&mut self, universe: &mut Universe<C, E, M, CA, EA>) -> Option<SM>;
    // called for every message received from a client.
    fn handle_client_message(&mut Server<C, E, M, CA, EA, SM, CM, Self>, &CM) -> Option<Vec<SM>>;
}

pub struct Server<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>,
    SM: Message, CM: Message, L: ServerLogic<C, E, M, CA, EA, SM, CM>
> {
    pub universe_len: usize,
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
}

impl<
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static,
    CA: CellAction<C> + 'static, EA: EntityAction<C, E> + 'static,
    SM: Message + 'static, CM: Message + 'static, L: ServerLogic<C, E, M, CA, EA, SM, CM> + 'static
> Server<C, E, M, CA, EA, SM, CM, L> {
    pub fn new(universe_size: usize, ws_host: &'static str, logic: L, seq: Arc<AtomicU32>) -> Box<Self> {
        let server = Box::new(Server {
            universe_len: universe_size * universe_size,
            logic,
            ws_broadcaster: unsafe { mem::uninitialized() },
            seq,
            __phantom_c: PhantomData,
            __phantom_e: PhantomData,
            __phantom_m: PhantomData,
            __phantom_ca: PhantomData,
            __phantom_ea: PhantomData,
            __phantom_sm: PhantomData,
            __phamtom_cm: PhantomData,
        });

        // get a pointer to the inner server and use it to initialize the websocket server
        let server_ptr = Box::into_raw(server);
        unsafe {
            let server_ref: &mut Server<C, E, M, CA, EA, SM, CM, L> = &mut *server_ptr;
            ptr::write(&mut server_ref.ws_broadcaster as *mut ws::Sender, init_ws_server(ws_host, Spaceship(server_ptr)));
            Box::from_raw(server_ptr)
        }
    }
}

impl<
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static,
    CA: CellAction<C> + 'static, EA: EntityAction<C, E> + 'static,
    SM: Message + 'static, CM: Message + 'static, L: ServerLogic<C, E, M, CA, EA, SM, CM> + 'static
> Server<C, E, M, CA, EA, SM, CM, L> {
    pub fn get_seq(&self) -> u32 {
        self.seq.load(Ordering::Relaxed)
    }
}

impl<
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static,
    CA: CellAction<C> + 'static, EA: EntityAction<C, E> + 'static, N: Engine<C, E, M, CA, EA>,
    SM: Message + 'static, CM: Message + 'static, L: ServerLogic<C, E, M, CA, EA, SM, CM> + 'static
> Middleware<C, E, M, CA, EA, N> for Box<Server<C, E, M, CA, EA, SM, CM, L>> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        if let Some(msg) = self.logic.tick(universe) {
            // convert the message into binary format and then send it over the websocket
            match self.ws_broadcaster.send::<&[u8]>(msg.bin_serialize().unwrap().as_slice().into()) {
                Err(err) => println!("Error while sending message over the websocket: {:?}", err),
                _ => (),
            }
        }
        self.seq.fetch_add(1, Ordering::Relaxed);
    }
}

struct WsServerHandler<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>,
    SM: Message, CM: Message, L: ServerLogic<C, E, M, CA, EA, SM, CM>
>  {
    out: ws::Sender,
    server_ptr: Spaceship<Server<C, E, M, CA, EA, SM, CM, L>>,
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>,
    SM: Message, CM: Message, L: ServerLogic<C, E, M, CA, EA, SM, CM>
> WsServerHandler<C, E, M, CA, EA, SM, CM, L> {
    pub fn new(out: ws::Sender, server_ptr: Spaceship<Server<C, E, M, CA, EA, SM, CM, L>>) -> Self {
        WsServerHandler { out, server_ptr }
    }
}

struct Spaceship<T>(*mut T);

impl<T> Clone for Spaceship<T> {
    fn clone(&self) -> Self {
        Spaceship(self.0)
    }
}

unsafe impl<T> Send for Spaceship<T> {}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>,
    SM: Message, CM: Message, L: ServerLogic<C, E, M, CA, EA, CM, SM>
> Handler for WsServerHandler<C, E, M, CA, EA, CM, SM, L> {
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

                let server: &mut Server<C, E, M, CA, EA, CM, SM, L> = unsafe { &mut *self.server_ptr.0 };
                match L::handle_client_message(server, &client_msg) {
                    Some(msgs) => {
                        // serialize and transmit the messages to the client
                        for msg in msgs {
                            let serialized: Vec<u8> = msg.bin_serialize().expect("Unable to send message to client!");
                            self.out.send::<&[u8]>(serialized.as_slice().into()); // TODO: Look into handling errors
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
    C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static,
    CA: CellAction<C> + 'static, EA: EntityAction<C, E> + 'static,
    SM: Message + 'static, CM: Message + 'static, L: ServerLogic<C, E, M, CA, EA, SM, CM> + 'static
> (
    ws_host: &'static str, ship: Spaceship<Server<C, E, M, CA, EA, SM, CM, L>>
) -> ws::Sender {
    let server = WebSocket::new(move |out: ws::Sender| {
        WsServerHandler::new(out, ship.clone())
    }).expect("Unable to initialize websocket server!");

    let broadcaster = server.broadcaster();

    // start the server on a separate thread
    thread::spawn(move || {
        server.listen(ws_host).expect("Unable to initialize websocket server!");
    });

    broadcaster
}
