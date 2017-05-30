//! Minutiae Simulation Engine

#![feature(closure_to_fn_coercion, conservative_impl_trait, integer_atomics, test)]

extern crate test;
extern crate num_cpus;
extern crate uuid;

#[cfg(any(feature = "serde", feature = "client"))]
extern crate serde;

#[macro_use]
#[cfg(any(feature = "serde", feature = "client"))]
extern crate serde_derive;

#[cfg(any(feature = "server", feature = "client"))]
extern crate bincode;

#[cfg(any(feature = "server", feature = "client"))]
extern crate flate2;

#[cfg(feature = "server")]
extern crate ws;

pub mod universe;
pub mod container;
pub mod cell;
pub mod entity;
pub mod action;
pub mod engine;
pub mod generator;
pub mod util;
pub mod driver;
#[cfg(any(feature = "server", feature = "client"))]
pub mod server;
// #[cfg(target_os = "emscripten")]
pub mod emscripten;


pub mod prelude {
    //! Utility module for re-exporting some commonly used traits
    pub use universe::{Universe, UniverseConf};
    pub use entity::{Entity, EntityState, MutEntityState};
    pub use cell::{Cell, CellState};
    pub use action::{Action, CellAction, SelfAction, EntityAction, OwnedAction};
    pub use generator::Generator;
    pub use engine::Engine;
    pub use driver::Driver;
    pub use driver::middleware::Middleware;
    pub use container::EntityContainer;
}
