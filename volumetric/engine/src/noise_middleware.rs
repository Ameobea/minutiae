//! Defines a middleware that sets the state of every cell in the universe equal the output of a noise function.

use minutiae::prelude::*;
use noise::{NoiseModule, Point3};

use super::*;
use buf3d_middleware::BufColumn;

/// Configuration status and state for the entire backend.
pub struct MasterConf {
    pub needs_resize: bool,
    pub canvas_size: usize,
    pub zoom: f32,
    pub speed: f32,
}

impl Default for MasterConf {
    fn default() -> Self {
            MasterConf {
            needs_resize: false,
            canvas_size: 0,
            speed: 0.00758,
            zoom: 0.0132312,
        }
    }
}

/// given a buffer containing all of the cells in the universe, calculates values for each of them using
/// perlin noise and sets their states according to the result.
fn drive_noise<C: CellState + BufColumn>(
    cells_buf: &mut [Cell<C>], seq: usize, noise: &NoiseModule<Point3<f32>, Output=f32>,
    universe_size: usize, zoom: f32, speed: f32
) {
    for y in 0..universe_size {
        for x in 0..universe_size {
            let column = cells_buf[(y * universe_size) + x].state.get_col_mut();
            for z in 0..universe_size {
                // calculate noise value for current coordinate and sequence number
                let val = noise.get([
                    (x as f32 + (seq as f32 * 5.8)) * zoom,
                    (y as f32) * zoom,
                    (z as f32) * speed,
                ]);

                // set the cell's state equal to that value
                column[z] = val as f32;
            }
        }
    }
}

/// Very custom function for changing the size of the universe by either removing elements from it or expanding
/// it with elements to match the new length.  Totally ignores all entity-related stuff for now and will almost
/// certainly break if entities are utilized in any way.
fn resize_universe<
    C: CellState + BufColumn, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, G: Engine<C, E, M, CA, EA>
>(universe: &mut Universe<C, E, M, CA, EA>, new_size: usize) {
    if new_size == 0 {
        return error("Requested change of universe size to 0!");
    }

    // universe.cells.resize(new_size * new_size, Cell { state: (0.0).into() } );
    universe.conf.size = new_size;
}

pub struct NoiseStepper<N: NoiseModule<Point3<f32>>> {
    conf: MasterConf,
    noise: N,
    universe_size: usize,
}

impl<
    C: CellState + BufColumn,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    G: Engine<C, E, M, CA, EA>,
    N: NoiseModule<Point3<f32>, Output=f32>
> Middleware<C, E, M, CA, EA, G> for NoiseStepper<N> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        // handle any new setting changes before rendering

        if universe.seq % ((60 * 4) + 1) == 0 || universe.seq == 1 {
            drive_noise(
                &mut universe.cells,
                universe.seq,
                &self.noise,
                self.universe_size,
                self.conf.zoom,
                self.conf.speed
            );
        }
    }
}

impl<N: NoiseModule<Point3<f32>>> NoiseStepper<N> {
    pub fn new(noise: N, conf: Option<MasterConf>, universe_size: usize) -> Self {
        NoiseStepper {
            conf: match conf {
                Some(c) => c,
                None => MasterConf::default(),
            },
            noise,
            universe_size,
        }
    }
}
