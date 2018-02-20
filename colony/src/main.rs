extern crate minutiae;

use minutiae::prelude::*;

pub mod sparse_universe;
pub mod world_generator;

pub const UNIVERSE_SIZE: usize = 100_000;

#[derive(Clone)]
pub enum CS {

}

impl CellState for CS {}

#[derive(Clone)]
pub enum ES {

}

#[derive(Clone, Default, Copy)]
pub struct MES {

}

impl MutEntityState for MES{}

impl EntityState<CS> for ES {}

pub enum CA {

}

impl CellAction<CS> for CA {}

pub enum EA {

}

impl EntityAction<CS, ES> for EA {}



fn main() {
    println!("Hello, world!");
}
