//! Minutae Simulation Engine

#![feature(conservative_impl_trait, test)]

extern crate test;
extern crate rayon;
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
