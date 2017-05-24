//! Sets up code for communicating changes in universe state with remote clients.

use std::{mem, ptr, thread};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

use ws::{self, WebSocket, Handler, Message};

use minutae::universe::Universe;
use minutae::container::EntityContainer;
use minutae::cell::{Cell, CellState};
use minutae::entity::{EntityState, MutEntityState};
use minutae::action::{CellAction, EntityAction};
use minutae::engine::Engine;
use minutae::driver::middleware::Middleware;
use minutae_libremote::{Color, ClientMessage, ClientMessageContent, Diff, ServerMessage, ServerMessageContents};

pub struct ColorServer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    universe_len: usize,
    last_colors: RwLock<Vec<Color>>,
    diffs: Vec<Diff>,
    color_calculator: fn(&Cell<C>, entity_indexes: &[usize], entity_container: &EntityContainer<C, E, M>) -> Color,
    ws_broadcaster: ws::Sender,
    seq: AtomicU32,
}

struct WsServerHandler<C: CellState, E: EntityState<C>, M: MutEntityState> {
    out: ws::Sender,
    colorserver_ptr: Spaceship<ColorServer<C, E, M>>,
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> WsServerHandler<C, E, M> {
    pub fn new(out: ws::Sender, colorserver_ptr: Spaceship<ColorServer<C, E, M>>) -> Self {
        WsServerHandler { out, colorserver_ptr }
    }
}

struct Spaceship<T>(*const T);

impl<T> Clone for Spaceship<T> {
    fn clone(&self) -> Self {
        Spaceship(self.0)
    }
}

unsafe impl<T> Send for Spaceship<T> {}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> Handler for WsServerHandler<C, E, M> {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        match msg {
            ws::Message::Binary(bin) => {
                // try to convert the received message into a `ClientMessage`
                let client_msg = match ClientMessage::deserialize(&bin) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("Error deserializing `ClientMessage` from binary data sent from user: {:?}", err);
                        return Ok(())
                    }
                };

                match client_msg.content {
                    ClientMessageContent::SendSnapshot => {
                        println!("Received request to send snapshot...");
                        // calculate the snapshot by pulling the data from the `ColorServer` pointer
                        let server: &ColorServer<C, E, M> = unsafe { &*self.colorserver_ptr.0 };
                        let colors = server.last_colors.read().expect("Unable to lock colors vector for reading!").clone();
                        let msg = ServerMessage {
                            seq: server.seq.load(Ordering::Relaxed),
                            contents: ServerMessageContents::Snapshot(colors),
                        }.serialize().expect("Unable to serialize snapshot message!");
                        return self.out.send::<&[u8]>(&msg)
                    },
                    _ => unimplemented!(),
                }
            },
            ws::Message::Text(text) => println!("Someone tried to send a text message over the WebSocket: {}", text),
        }

        Ok(())
    }
}

fn init_ws_server<C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static>(
    ws_host: &'static str, ship: Spaceship<ColorServer<C, E, M>>
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

impl<C: CellState + 'static, E: EntityState<C> + 'static, M: MutEntityState + 'static> ColorServer<C, E, M> {
    pub fn new(
        universe_size: usize, color_calculator: fn(
            &Cell<C>, entity_indexes: &[usize],
            entity_container: &EntityContainer<C, E, M>
        ) -> Color, ws_host: &'static str,
    ) -> Box<Self> { // boxed so we're sure it doesn't move and we can pass pointers to it around
        let server = Box::new(ColorServer {
            universe_len: universe_size * universe_size,
            last_colors: RwLock::new(vec![Color([0, 0, 0]); universe_size * universe_size]),
            diffs: Vec::new(),
            color_calculator: color_calculator,
            ws_broadcaster: unsafe { mem::uninitialized() },
            seq: AtomicU32::new(1),
        });

        // get a pointer to the inner server and use it to initialize the websocket server
        let server_ptr = Box::into_raw(server);
        unsafe {
            let server_ref: &mut ColorServer<C, E, M> = &mut *server_ptr;
            ptr::write(&mut server_ref.ws_broadcaster as *mut ws::Sender, init_ws_server(ws_host, Spaceship(server_ptr)));
            Box::from_raw(server_ptr)
        }
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, N> for Box<ColorServer<C, E, M>> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        // TODO: Create an option for making this parallel because it's a 100% parallelizable task
        let mut diffs = Vec::new();
        let mut colors = self.last_colors.write().expect("Unable to lock colors vector for writing!");
        for i in 0..self.universe_len {
            let cell = unsafe { universe.cells.get_unchecked(i) };
            let entity_indexes = universe.entities.get_entities_at(i);

            let new_color = (self.color_calculator)(cell, entity_indexes, &universe.entities);
            let mut last_color = unsafe { colors.get_unchecked_mut(i) };
            if &new_color != last_color {
                // color for that coordinate has changed, so add a diff to the diff buffer and update `last_colors`
                /*self.*/diffs.push(Diff {universe_index: i, color: new_color.clone()});
                (*last_color) = new_color;
            }
        }

        // create a `ServerMessage` out of the diffs, serialize/compress it, and broadcast it to all connected clients
        let msg = ServerMessage {
            seq: self.seq.load(Ordering::Relaxed),
            contents: ServerMessageContents::Diff(diffs),
        }.serialize().expect("Unable to convert `ServerMessage` into binary!");
        let ws_msg: Message = msg.into();
        self.ws_broadcaster.broadcast(ws_msg).expect("Unable to send message over websocket!");

        // finally, clear the buffer so it's fresh for the next iteration
        // self.diffs.clear();
        self.seq.fetch_add(1, Ordering::Relaxed);
    }
}
