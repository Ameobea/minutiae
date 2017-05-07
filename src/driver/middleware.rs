//! Declares additions that can be added onto the driver either before or after a render completes.  Enables things like
//! rendering, state storage, etc.

use std::fmt::Display;
use std::thread;
use std::time::Duration;

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use engine::Engine;

/// Adds some side effect on to the end or beginning of the render cycle
pub trait Middleware<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> {
    fn after_render(&mut self, _: &Universe<C, E, M, CA, EA>) {}

    fn before_render(&mut self, _: &Universe<C, E, M, CA, EA>) {}
}


pub struct UniverseDisplayer {}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, N> for UniverseDisplayer where C:Display, E:Display {
    fn after_render(&mut self, universe: &Universe<C, E, M, CA, EA>) {
        println!("{:?}", universe);
    }
}

pub struct Delay(pub u64);

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, N> for Delay {
    fn before_render(&mut self, _: &Universe<C, E, M, CA, EA>) {
        thread::sleep(Duration::from_millis(self.0))
    }
}
