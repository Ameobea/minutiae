//! Middleware with the purpose of taking the data from universe and converting it into a format that can be
//! passed into WebGL for volumetric rendering.

use minutiae::prelude::*;

// use super::*;

pub trait BufColumn {
    fn get_col(&self) -> &[f32];
    fn get_col_mut(&mut self) -> &mut [f32];
}

type RenderCb = unsafe extern fn(*const f32);

pub struct Buf3dWriter {
    buf: Vec<f32>,
    cb: RenderCb
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

        // execute the callback with the pointer to the updated buffer
        unsafe { (self.cb)(self.buf.as_ptr()) }
    }
}

impl Buf3dWriter {
    pub fn new(universe_size: usize, cb: RenderCb) -> Self {
        Buf3dWriter { buf: vec![0.0f32; universe_size * universe_size * universe_size], cb }
    }
}
