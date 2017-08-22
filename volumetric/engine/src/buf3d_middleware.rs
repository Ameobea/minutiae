//! Middleware with the purpose of taking the data from universe and converting it into a format that can be
//! passed into WebGL for volumetric rendering.

use std::f64;

use minutiae::prelude::*;
use noise::Point3;

use super::debug;

pub trait BufColumn {
    fn get_col(&self) -> &[f32];
    fn get_col_mut(&mut self) -> &mut [f32];
}

type RenderCb = unsafe extern fn(*const f32, f64, f64, f64, f64, f64, f64, f64);

pub struct Buf3dWriter {
    buf: Vec<f32>,
    cb: RenderCb,
    screen_ratio: f64,
    camera_coord: Point3<f64>,
    focal_coord: Point3<f64>,
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, G: Engine<C, E, M, CA, EA>
// require that supplied `CellState` can be converted into a row of the buffer
> Middleware<C, E, M, CA, EA, G> for Buf3dWriter where C: BufColumn {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        // populate the buffer with the values from each vector of Z values
        for (ix, stack) in universe.cells
            .iter()
            .map(|cell| cell.state.get_col()) // fetch the slice of Z `f32`s
            .enumerate() {
            for (z, val) in stack.iter().enumerate() {
                self.buf[(ix * stack.len()) + z] = *val;
            }
        }

        const STEPS_PER_ORBIT: usize = 128;
        // pivot the camera around the origin
        let cur_step = universe.seq % STEPS_PER_ORBIT;
        let cur_rads = (cur_step as f64 / STEPS_PER_ORBIT as f64) * 2. * f64::consts::PI;
        let camera_coord = [4., cur_rads.cos() * 4., cur_rads.sin() * 4.];
        debug(&format!("Camera coord: {:?}", camera_coord));

        // execute the callback with the pointer to the updated buffer
        unsafe { (self.cb)(
            self.buf.as_ptr(), self.screen_ratio, /*self.*/camera_coord[0], /*self.*/camera_coord[1], /*self.*/camera_coord[2],
            self.focal_coord[0], self.focal_coord[1], self.focal_coord[2]
        ) }
    }
}

impl Buf3dWriter {
    pub fn new(
        universe_size: usize, cb: RenderCb, screen_ratio: f64, camera_coord: Point3<f64>,
        focal_coord: Point3<f64>
    ) -> Self {
        Buf3dWriter {
            buf: vec![0.0f32; universe_size * universe_size * universe_size],
            cb,
            screen_ratio,
            camera_coord,
            focal_coord,
        }
    }
}
