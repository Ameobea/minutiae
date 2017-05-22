//! Sets up code for communicating changes in universe state with remote clients.

use minutae::universe::Universe;
use minutae::container::EntityContainer;
use minutae::cell::{Cell, CellState};
use minutae::entity::{EntityState, MutEntityState};
use minutae::action::{CellAction, EntityAction};
use minutae::engine::Engine;
use minutae::driver::middleware::Middleware;
use minutae_libremote::{Color, Diff};

struct ColorDiffCalculator<C: CellState, E: EntityState<C>, M: MutEntityState> {
    universe_len: usize,
    last_colors: Vec<Color>,
    diffs: Vec<Diff>,
    color_calculator: fn(&Cell<C>, entity_indexes: &[usize], entity_container: &EntityContainer<C, E, M>) -> Color,
    diff_handler: fn(&[Diff]),
}

impl<C: CellState, E: EntityState<C>, M: MutEntityState> ColorDiffCalculator<C, E, M> {
    pub fn new(
        universe_size: usize, color_calculator: fn(&Cell<C>, entity_indexes: &[usize],
        entity_container: &EntityContainer<C, E, M>) -> Color, diff_handler: fn(&[Diff])
    ) -> Self {
        ColorDiffCalculator {
            universe_len: universe_size * universe_size,
            last_colors: vec![Color([0, 0, 0]); universe_size * universe_size],
            diffs: Vec::new(),
            color_calculator: color_calculator,
            diff_handler: diff_handler,
        }
    }
}

impl<
    C: CellState, E: EntityState<C>, M: MutEntityState, CA: CellAction<C>, EA: EntityAction<C, E>, N: Engine<C, E, M, CA, EA>
> Middleware<C, E, M, CA, EA, N> for ColorDiffCalculator<C, E, M> {
    fn after_render(&mut self, universe: &mut Universe<C, E, M, CA, EA>) {
        for i in 0..self.universe_len {
            let cell = unsafe { universe.cells.get_unchecked(i) };
            let entity_indexes = universe.entities.get_entities_at(i);

            let mut last_color = unsafe { self.last_colors.get_unchecked_mut(i) };
            let new_color = (self.color_calculator)(cell, entity_indexes, &universe.entities);
            if &new_color != last_color {
                // color for that coordinate has changed, so add a diff to the diff buffer and update `last_colors`
                self.diffs.push(Diff {universe_index: i, color: new_color.clone()});
                (*last_color) = new_color;
            }
        }

        // call the diff handler with the collected diffs
        (self.diff_handler)(&self.diffs);
        // and then clear the buffer
        self.diffs.clear();
    }
}
