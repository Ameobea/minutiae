//! Minutae Simulation Engine

#![feature(conservative_impl_trait, test)]

extern crate test;
extern crate rayon;

pub mod universe;
pub mod cell;
pub mod entity;
pub mod action;
pub mod engine;
pub mod generator;
pub mod util;
pub mod driver;
