#![feature(conservative_impl_trait, integer_atomics, test)]

#[macro_use]
extern crate lazy_static;
extern crate minutiae;
extern crate noise;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;
extern crate uuid;

use minutiae::prelude::*;
use minutiae::emscripten::UserEvent;
use minutiae::server::Tys;
use minutiae::server::Event;
use minutiae::server::{HybridClientMessage, HybridServerMessage};
use minutiae::util::{get_index, Color};
use minutiae::universe::Universe2D;

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
    type ClientMessage = HybridClientMessage;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ColonyEvent {
    Splat(P2D, [u8; 4]),
}

impl Event<ColonyTys> for ColonyEvent {
    fn apply(&self, universe: &mut <ColonyTys as Tys>::U) {
        match self {
            &ColonyEvent::Splat(P2D { x, y }, color) => {
                for y in y-10..y+10 {
                    for x in x-10..x+10 {
                        let state = CS::Color(color);
                        universe.set_cell_unchecked(get_index(x, y, UNIVERSE_SIZE), state);
                    }
                }
            }
        }
    }
}

// pub fn handle_user_event(event: UserEvent) -> ColonyTys::ClientMessage
