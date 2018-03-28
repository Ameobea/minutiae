//! Minutiae Simulation Engine

#![allow(unused_features)]

#![feature(associated_type_defaults, box_syntax, conservative_impl_trait, core_intrinsics, entry_or_default, integer_atomics, never_type, nll, step_trait, test)]

extern crate test;
extern crate num_cpus;
extern crate uuid;
#[macro_use]
extern crate lazy_static;
extern crate futures;

#[cfg(any(feature = "serde", feature = "client"))]
extern crate serde;

#[macro_use]
#[cfg(any(feature = "serde", feature = "client"))]
extern crate serde_derive;

#[cfg(any(feature = "server", feature = "client"))]
extern crate bincode;

#[cfg(any(feature = "server", feature = "client"))]
extern crate flate2;

extern crate gif;

#[cfg(feature = "server")]
extern crate websocket;
#[cfg(feature = "server")]
extern crate tokio_core;
#[cfg(feature = "server")]
extern crate futures_cpupool;

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
pub mod emscripten;

pub mod prelude {
    //! Utility module for re-exporting some commonly used traits
    pub use universe::{Universe, Universe2DConf, Universe2DConf as UniverseConf};
    pub use entity::{Entity, EntityState, MutEntityState};
    pub use cell::{Cell, CellState};
    pub use action::{Action, CellAction, SelfAction, EntityAction, OwnedAction};
    pub use generator::Generator;
    pub use engine::Engine;
    pub use driver::Driver;
    pub use driver::middleware::Middleware;
    pub use container::EntityContainer;
    pub use util::*;
}
