use minutiae::prelude::*;

use sparse_universe::CellGenerator;
use super::*;

#[derive(Clone)]
pub struct WorldGenerator;

impl<I: Ord> CellGenerator<CS, ES, MES, I> for WorldGenerator {
    fn gen_cell(&self, universe_index: I) -> Cell<CS> {
        Cell { state: CS::__placeholder }
    }

    fn gen_initial_entities(&self, universe_index: I) -> Vec<Entity<CS, ES, MES>> {
        Vec::new()
    }
}
