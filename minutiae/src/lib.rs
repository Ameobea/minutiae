//! Minutiae Simulation Engine

#![feature(closure_to_fn_coercion, conservative_impl_trait, test)]

extern crate test;
extern crate num_cpus;
extern crate uuid;

#[cfg(feature = "serde")]
extern crate serde;
#[macro_use]
#[cfg(feature = "serde")]
extern crate serde_derive;

pub mod universe;
pub mod container;
pub mod cell;
pub mod entity;
pub mod action;
pub mod engine;
pub mod generator;
pub mod util;
pub mod driver;
