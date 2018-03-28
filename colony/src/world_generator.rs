use minutiae::prelude::*;
use noise::{Fbm, NoiseFn};

use sparse_universe::CellGenerator;
use super::*;

lazy_static! {
    static ref NOISE: Fbm = Fbm::new();
}

const ZOOM: f64 = 0.01;

#[derive(Copy, Clone)]
pub struct WorldGenerator;

impl CellGenerator<CS, ES, MES, P2D> for WorldGenerator {
    fn gen_cell(P2D {x, y}: P2D) -> Cell<CS> {
        // if x % 40 == 0 || y % 40 == 10 {
        //     Cell { state: CS::Empty }
        // } else {
        //     Cell { state: CS::Color([((x % 10) * 20) as u8, ((y % 10) * 20) as u8, ((x+y) % 50) as u8, 255u8]) }
        // }

        let noise = NOISE.get([(x as f64) * ZOOM, (y as f64) * ZOOM]);
        let state: CS = if noise <= 0.0 {
            CS::Empty
        } else {
            let color = if noise <= 0.5 { [255, 130, 30, 255] } else { [39, 244, 139, 255] };
            CS::Color(color)
        };

        Cell { state }
    }

    fn gen_initial_entities(universe_index: P2D) -> Vec<Entity<CS, ES, MES>> {
        Vec::new()
    }
}
