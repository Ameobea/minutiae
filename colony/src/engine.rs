use minutiae::prelude::*;

use sparse_universe::{CellGenerator, P2D, Sparse2DUniverse};
use super::*;

pub struct ColonyEngine;

impl<
    G: CellGenerator<CS, ES, MES, P2D>
> Engine<CS, ES, MES, CA, EA, Sparse2DUniverse<CS, ES, MES, G>> for ColonyEngine {
    fn step(&mut self, universe: &mut Sparse2DUniverse<CS, ES, MES, G>) {
        unimplemented!();
    }
}
