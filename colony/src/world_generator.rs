use minutiae::prelude::*;

use sparse_universe::CellGenerator;
use super::*;

#[derive(Copy, Clone)]
pub struct WorldGenerator;

impl CellGenerator<CS, ES, MES, P2D> for WorldGenerator {
    fn gen_cell(P2D {x, y}: P2D) -> Cell<CS> {
        if x % 40 == 0 || y % 40 == 10 {
            Cell { state: CS::Empty }
        } else {
            Cell { state: CS::Color([((x % 10) * 20) as u8, ((y % 10) * 20) as u8, ((x+y) % 50) as u8, 255u8]) }
        }
    }

    fn gen_initial_entities(universe_index: P2D) -> Vec<Entity<CS, ES, MES>> {
        Vec::new()
    }
}
