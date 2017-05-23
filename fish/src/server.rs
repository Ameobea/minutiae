//! Sets up code for communicating changes in universe state with remote clients.

use std::thread;

use ws::{self, WebSocket, connect, Handler, Message};

use minutae::universe::Universe;
use minutae::container::EntityContainer;
use minutae::cell::{Cell, CellState};
use minutae::entity::{EntityState, MutEntityState};
use minutae::action::{CellAction, EntityAction};
use minutae::engine::Engine;
use minutae::driver::middleware::Middleware;
use minutae_libremote::{Color, Diff, ServerMessage, ServerMessageContents};

struct ColorServer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    universe_len: usize,
    last_colors: Vec<Color>,
    diffs: Vec<Diff>,
    color_calculator: fn(&Cell<C>, entity_indexes: &[usize], entity_container: &EntityContainer<C, E, M>) -> Color,
    ws_broadcaster: ws::Sender,
    seq: u32,
}

/// Holds the WebSocket server's state
struct WsServerHandler {
    out: ws::Sender,
}

impl WsServerHandler {
    pub fn new(out: ws::Sender) -> Self {
        WsServerHandler { out }
    }
}

impl Handler for WsServerHandler {
    fn on_message(&mut self, msg: ws::Message) -> Result<(), ws::Error> {
        unimplemented!(); // TODO
    }
}

fn init_ws_server(ws_host: &'static str) -> ws::Sender {
    let server = WebSocket::new(move |out: ws::Sender| {
        WsServerHandler::new(out)
    }).expect("Unable to initialize websocket server!");

    let broadcaster = server.broadcaster();

    // start the server on a separate thread
    thread::spawn(move || {
        server.listen(ws_host).expect("Unable to initialize websocket server!");
    });

    broadcaster
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> ColorServer<C, E, M> {
    pub fn new(
        universe_size: usize, color_calculator: fn(
            &Cell<C>, entity_indexes: &[usize],
            entity_container: &EntityContainer<C, E, M>
        ) -> Color, diff_handler: fn(&[Diff]), ws_host: &'static str,
    ) -> Self {
        ColorServer {
            universe_len: universe_size * universe_size,
            last_colors: vec![Color([0, 0, 0]); universe_size * universe_size],
            diffs: Vec::new(),
            color_calculator: color_calculator,
            ws_broadcaster: init_ws_server(ws_host),
            seq: 1,
        }
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, N> for ColorServer<C, E, M> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        // TODO: Create an option for making this parallel because it's a 100% parallelizable task
        let mut diffs = Vec::new();
        for i in 0..self.universe_len {
            let cell = unsafe { universe.cells.get_unchecked(i) };
            let entity_indexes = universe.entities.get_entities_at(i);

            let mut last_color = unsafe { self.last_colors.get_unchecked_mut(i) };
            let new_color = (self.color_calculator)(cell, entity_indexes, &universe.entities);
            if &new_color != last_color {
                // color for that coordinate has changed, so add a diff to the diff buffer and update `last_colors`
                /*self.*/diffs.push(Diff {universe_index: i, color: new_color.clone()});
                (*last_color) = new_color;
            }
        }

        // create a `ServerMessage` out of the diffs, serialize/compress it, and broadcast it to all connected clients
        let msg = ServerMessage {
            seq: self.seq,
            contents: ServerMessageContents::Diff(diffs),
        }.serialize().expect("Unable to convert `ServerMessage` into binary!");
        let ws_msg: Message = msg.into();
        self.ws_broadcaster.broadcast(ws_msg).expect("Unable to send message over websocket!");

        // finally, clear the buffer so it's fresh for the next iteration
        // self.diffs.clear();
        self.seq += 1;
    }
}
