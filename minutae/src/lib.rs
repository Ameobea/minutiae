//! Minutae Simulation Engine

#![feature(associated_consts, conservative_impl_trait, slice_patterns, test)]

extern crate test;
extern crate futures;
extern crate num_cpus;
extern crate smallvec;
extern crate uuid;

pub mod universe;
pub mod container;
pub mod cell;
pub mod entity;
pub mod action;
pub mod engine;
pub mod generator;
pub mod util;
pub mod driver;
