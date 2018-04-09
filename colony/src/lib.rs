#![feature(box_syntax, conservative_impl_trait, integer_atomics, test)]

#[macro_use]
extern crate lazy_static;
extern crate minutiae;
extern crate noise;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;
extern crate uuid;

use std::mem::transmute;

use minutiae::prelude::*;
use minutiae::emscripten::UserEvent;
use minutiae::server::Tys;
use minutiae::server::Event;
use minutiae::server::{ClientEventAction, HybridClientMessage, HybridClientMessageContents, HybridServerMessage, HybridServerMessageContents};
use minutiae::util::{get_index, Color};
use minutiae::universe::Universe2D;
use uuid::Uuid;

pub mod engine;
pub mod entity_driver;
pub mod sparse_universe;
pub mod world_generator;

use sparse_universe::P2D;
use world_generator::WorldGenerator;

pub const UNIVERSE_SIZE: usize = 800;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CS {
    Empty,
    Color([u8; 4]),
}

impl Default for CS {
    fn default() -> Self {
        CS::Empty
    }
}

impl CellState for CS {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ES {
    __placeholder,
}

#[derive(Clone, Default, Copy, Serialize, Deserialize)]
pub struct MES {
    __placeholder: u32,
}

impl MutEntityState for MES {}

impl EntityState<CS> for ES {}

#[derive(Clone, Debug)]
pub enum CA {
    __placeholder,
}

impl CellAction<CS> for CA {}

#[derive(Clone, Debug)]
pub enum EA {
    __placeholder,
}

impl EntityAction<CS, ES> for EA {}

pub fn color_calculator(
    cell: &Cell<CS>,
    _entity_indexes: &[usize],
    _entity_container: &EntityContainer<CS, ES, MES, usize>
) -> [u8; 4] {
    match cell.state {
        CS::Empty => [0, 0, 0, 255],
        CS::Color(color) => color,
    }
}

#[derive(Clone, Copy)]
pub struct ColonyTys;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CustomClientMessage {
    Click { x: u32, y: u32, color: [u8; 4] },
}

impl Tys for ColonyTys {
    type C = CS;
    type E = ES;
    type M = MES;
    type CA = CA;
    type EA = EA;
    type I = usize;
    type U = Universe2D<CS, ES, MES>;
    type V = ColonyEvent;
    type Snapshot = Self::U;
    type ServerMessage = HybridServerMessage<Self>;
    type ClientMessage = HybridClientMessage<CustomClientMessage>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ColonyEvent {
    Splat(P2D, [u8; 4]),
}

fn draw_spot(universe: &mut <ColonyTys as Tys>::U, x: usize, y: usize, color: [u8; 4]) {
    if x < 10 || y < 10 || x >= (UNIVERSE_SIZE - 10) || y >= (UNIVERSE_SIZE - 10) {
        return; // Out of bounds; just draw nothing.
    }

    for y in y-10..y+10 {
        for x in x-10..x+10 {
            let state = CS::Color(color);
            universe.set_cell_unchecked(get_index(x, y, UNIVERSE_SIZE), state);
        }
    }
}

impl Event<ColonyTys> for ColonyEvent {
    fn apply(&self, universe: &mut <ColonyTys as Tys>::U) {
        match self {
            &ColonyEvent::Splat(P2D { x, y }, color) => {
                draw_spot(universe, x, y, color)
            }
        }
    }
}

pub fn custom_event_handler(
    universe: &mut <ColonyTys as Tys>::U,
    seq: u32,
    custom_event: CustomClientMessage
) -> ClientEventAction<ColonyTys> {
    match custom_event {
        CustomClientMessage::Click{ x, y, color } => {
            draw_spot(universe, x as usize, y as usize, color);

            let event = ColonyEvent::Splat(P2D { x: x as usize, y: y as usize }, color);
            let server_msg = HybridServerMessage::new(seq, HybridServerMessageContents::Event(vec![event]));
            ClientEventAction {
                broadcast_msgs: Some(vec![server_msg]),
                response_msg: None,
            }
        },
    }
}

fn uuid_to_color(uuid: Uuid) -> [u8; 4] {
    let mut color: [u8; 4] = unsafe { transmute(uuid.as_fields().0) };
    color[3] = 255;
    color
}

pub fn map_user_event_to_client_message(client_uuid: Uuid, event: UserEvent) -> Option<HybridClientMessageContents<CustomClientMessage>> {
    match event {
        UserEvent::CanvasClick { x, y } => {
            let custom_evt = CustomClientMessage::Click { x, y, color: uuid_to_color(client_uuid) };
            Some(HybridClientMessageContents::Custom(custom_evt))
        },
        _ => None,
    }
}
