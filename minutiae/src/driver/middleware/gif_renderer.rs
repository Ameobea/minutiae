/// Creates a GIF image with a new frame create for each tick of the simulation.

use std::fs::File;

use gif::{Encoder, Frame, Repeat, SetParameter};

use action::{CellAction, EntityAction};
use cell::CellState;
use engine::Engine;
use entity::{EntityState, MutEntityState};
use universe::{Universe, Universe2D};
use util::ColorCalculator;
use super::Middleware;

pub struct GifRenderer<C: CellState, E: EntityState<C>, M: MutEntityState> {
    encoder: Encoder<File>,
    universe_size: u16,
    colorfn: ColorCalculator<C, E, M>,
}

impl<
    C: CellState,
    E: EntityState<C>,
    M: MutEntityState,
    CA: CellAction<C>,
    EA: EntityAction<C, E>,
    N: Engine<C, E, M, CA, EA, Universe2D<C, E, M>>,
> Middleware<C, E, M, CA, EA, Universe2D<C, E, M>, N> for GifRenderer<C, E, M> {
    fn after_render(&mut self, universe: &mut Universe2D<C, E, M>) {
        // calculate colors for each of the pixels in the universe and map it into the array format used by `gif`
        let mut pixels: Vec<u8> = Vec::with_capacity(self.universe_size as usize * self.universe_size as usize * 3);
        for i in 0..(self.universe_size as usize * self.universe_size as usize) {
            let entities = universe.get_entities().get_entities_at(i);
            let color = (self.colorfn)(&universe.cells[i], entities, &universe.entities);

            pixels.push(color[0]);
            pixels.push(color[1]);
            pixels.push(color[2]);
        }

        let frame = Frame::from_rgb(self.universe_size, self.universe_size, &pixels);
        self.encoder.write_frame(&frame).expect("Unable to write frame to output file!");
    }
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> GifRenderer<C, E, M> {
    pub fn new(output_path: &str, universe_size: usize, colorfn: ColorCalculator<C, E, M>) -> Self {
        let color_map = &[0xFF, 0xFF, 0xFF, 0, 0, 0];
        let image = File::create(output_path).unwrap();
        let mut encoder = Encoder::new(image, universe_size as u16, universe_size as u16, color_map).unwrap();
        encoder.set(Repeat::Infinite).unwrap();

        GifRenderer {
            encoder,
            universe_size: universe_size as u16,
            colorfn,
        }
    }
}
