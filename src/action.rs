//! Represents an entity's attempt to mutate the state of itself, another entity, or a cell.  These are returned
//! as the result of an entitie's transformation function and are run through the universe's simulation engine and
//! applied according to the rules set up there.

use entity::EntityState;
use cell::CellState;

pub struct Action<'a, C: CellState + 'a, E: EntityState<C> + 'a> {
    new_self_state: &'a mut EntityState<C>,
    new_cell_states: &'a [&'a mut CellState],
    new_entity_states: &'a [&'a mut E],
}
