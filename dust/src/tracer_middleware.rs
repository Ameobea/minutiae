//! Defines a middleware that creates tracer trails hilighting where particles have been.

use minutiae::prelude::*;
use palette::{Mix, Rgb};

use super::*;

pub struct TracerMiddleware{
    mix_factor: f32,
    fade_factor: f32,
    fade_interval: usize
}

impl TracerMiddleware {
    pub fn new(mix_factor: f32, fade_factor: f32, fade_interval: usize) -> Self {
        assert!(fade_interval > 0);
        assert!(mix_factor > 0.0);
        assert!(fade_factor > 0.0);

        TracerMiddleware { mix_factor, fade_factor, fade_interval }
    }
}

impl Middleware<
    CS, ES, MES, CA, EA, Box<ParallelEngine<CS, ES, MES, CA, EA, SerialGridIterator>>
> for TracerMiddleware {
    fn after_render(&mut self, universe: &mut Universe<CS, ES, MES, CA, EA>) {
        // for each of the entities, alter the color of the background to match the entity's color
        for (entity, entity_index, universe_index) in universe.entities.iter_mut() {
            let entity_color = entity.state.get_base_color();
            let mut cell = &mut universe.cells[universe_index];
            let cell_color = cell.state.color;
            let mixed_color = entity_color.mix(&entity_color, self.mix_factor);
            cell.state.color = mixed_color;
        }

        // fade the color of all cells every `n` ticks
        if universe.seq % self.fade_interval == 0 {
            for mut cell in &mut universe.cells {
                cell.state.color = cell.state.color.mix(&Rgb::new_u8(3, 3, 3), self.fade_factor);
            }
        }

        unsafe {
            ATTRACTION_FACTOR = {
                let period = 100;

                let progress: usize = universe.seq % (2 * period);
                if progress > period {
                    1.0 - ((progress - period) as f32 / period as f32)
                } else {
                    (progress as f32) / period as f32
                }
            };
        }
    }
}
