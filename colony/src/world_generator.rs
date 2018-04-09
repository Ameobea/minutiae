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

fn get_cs(noise: f64) -> CS {
    if noise <= -0.5 {
        CS::Empty
    } else if noise <= 0.0 {
        CS::Color([11, 33, 66, 255])
    } else if noise <= 0.5 {
        CS::Color([55, 3, 155, 255])
    } else {
        CS::Color([16, 135, 204, 255])
    }
}

impl CellGenerator<CS, ES, MES, P2D> for WorldGenerator {
    fn gen_cell(P2D {x, y}: P2D) -> Cell<CS> {
        // if x % 40 == 0 || y % 40 == 10 {
        //     Cell { state: CS::Empty }
        // } else {
        //     Cell { state: CS::Color([((x % 10) * 20) as u8, ((y % 10) * 20) as u8, ((x+y) % 50) as u8, 255u8]) }
        // }

        let noise = NOISE.get([(x as f64) * ZOOM, (y as f64) * ZOOM]);
        let state: CS = get_cs(noise);

        Cell { state }
    }

    fn gen_initial_entities(_universe_index: P2D) -> Vec<Entity<CS, ES, MES>> {
        Vec::new()
    }
}
