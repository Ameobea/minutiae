use minutiae::prelude::*;

use sparse_universe::CellGenerator;
use super::*;

pub struct WorldGenerator;

impl CellGenerator<CS, ES, MES> for WorldGenerator {
    fn gen_cell(&self, universe_index: usize) -> Cell<CS> {
        unimplemented!();
    }

    fn gen_initial_entities(&self, universe_index: usize) -> Vec<Entity<CS, ES, MES>> {
        Vec::new()
    }
}
