#![feature(conservative_impl_trait, integer_atomics, test)]

extern crate minutiae;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate test;
extern crate uuid;

use minutiae::prelude::*;
use minutiae::server::Tys;
use minutiae::server::Event;
use minutiae::server::HybridServerMessage;
use minutiae::util::Color;

pub mod engine;
pub mod entity_driver;
pub mod sparse_universe;
pub mod world_generator;

use sparse_universe::{P2D, Sparse2DUniverse};
use world_generator::WorldGenerator;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CS {
    Empty,
    Color(Color),
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
    __placeholder: usize,
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
    entity_indexes: &[usize],
    entity_container: &EntityContainer<CS, ES, MES, P2D>
) -> (u8, u8, u8) {
    match cell.state {
        CS::Empty => (0, 0, 0),
        CS::Color(color) => (color.0[0], color.0[1], color.0[2])
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
    type I = P2D;
    type U = Sparse2DUniverse<CS, ES, MES, WorldGenerator>;
    type V = ColonyEvent;
    type Snapshot = Self::U;
    type ServerMessage = HybridServerMessage<Self>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ColonyEvent {
    Splat(Color),
}

impl Event<ColonyTys> for ColonyEvent {
    fn apply(&self, universe: &mut <ColonyTys as Tys>::U) {
        // TODO
    }
}
