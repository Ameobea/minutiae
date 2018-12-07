//! Minutiae Simulation Engine

#![allow(unused_features)]
#![feature(
    associated_type_defaults,
    box_syntax,
    core_intrinsics,
    integer_atomics,
    never_type,
    nll,
    step_trait,
    test,
    thread_local,
    const_transmute
)]

extern crate futures;
extern crate num_cpus;
extern crate rand;
extern crate rand_pcg;
extern crate slab;
extern crate test;
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

extern crate gif;

#[cfg(feature = "server")]
extern crate futures_cpupool;
#[cfg(feature = "server")]
extern crate tokio_core;
#[cfg(feature = "server")]
extern crate websocket;

pub mod action;
pub mod cell;
pub mod container;
pub mod driver;
pub mod emscripten;
pub mod engine;
pub mod entity;
pub mod generator;
#[cfg(any(feature = "server", feature = "client"))]
pub mod server;
pub mod universe;
pub mod util;

pub mod prelude {
    //! Utility module for re-exporting some commonly used traits
    pub use action::{Action, CellAction, EntityAction, OwnedAction, SelfAction};
    pub use cell::{Cell, CellState};
    pub use container::EntityContainer;
    pub use driver::{middleware::Middleware, Driver};
    pub use engine::Engine;
    pub use entity::{Entity, EntityState, MutEntityState};
    pub use generator::Generator;
    pub use universe::{Universe, Universe2DConf, Universe2DConf as UniverseConf};
    pub use util::*;
}
