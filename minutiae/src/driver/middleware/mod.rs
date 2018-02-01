//! Declares additions that can be added onto the driver either before or after a render completes.  Enables things like
//! rendering, state storage, etc.

use std::fmt::Debug;
use std::thread;
use std::time::{Duration, Instant};

use universe::Universe;
use cell::CellState;
use entity::{EntityState, MutEntityState};
use action::{CellAction, EntityAction};
use engine::Engine;

pub mod gif_renderer;

/// Adds some side effect on to the end or beginning of the render cycle
pub trait Middleware<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M>,
    N: Engine<C, E, M, CA, EA, U>,
> {
    fn after_render(&mut self, _: &mut U) {}

    fn before_render(&mut self, _: &mut U) {}
}


pub struct UniverseDisplayer {}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M> + Debug,
    N: Engine<C, E, M, CA, EA, U>,
> Middleware<C, E, M, CA, EA, U, N> for UniverseDisplayer {
    fn after_render(&mut self, universe: &mut U) {
        println!("{:?}", universe);
    }
}

pub struct Delay(pub u64);

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M>,
    N: Engine<C, E, M, CA, EA, U>,
> Middleware<C, E, M, CA, EA, U, N> for Delay {
    fn before_render(&mut self, _: &mut U) {
        thread::sleep(Duration::from_millis(self.0))
    }
}

/// Checks the time between two ticks.  If the ticks
pub struct MinDelay {
    min_delay: Duration,
    last_tick: Instant,
}

impl MinDelay {
    pub fn new(min_delay_ms: u64) -> Self {
        MinDelay {
            min_delay: Duration::from_millis(min_delay_ms),
            last_tick: Instant::now(),
        }
    }

    /// Calculates a delay given the number of desired ticks per second
    pub fn from_tps(tps: f32) -> Self {
        let min_delay_ms: f32 = (1f32 / tps) * 1000f32;
        MinDelay {
            min_delay: Duration::from_millis(min_delay_ms as u64),
            last_tick: Instant::now(),
        }
    }
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    U: Universe<C, E, M>,
    N: Engine<C, E, M, CA, EA, U>,
> Middleware<C, E, M, CA, EA, U, N> for MinDelay {
    fn after_render(&mut self, _: &mut U) {
        let now = Instant::now();
        let time_diff: Duration = now - self.last_tick;

        self.last_tick = if time_diff < self.min_delay {
            thread::sleep(self.min_delay - time_diff);
            Instant::now()
        } else {
            now
        }
    }
}
