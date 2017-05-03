//! Declares a single cell of the universe.  Each cell has a coordinate that is made up of two unsigned integers and
//! represents its offset from the top left of the universe.  All cells are one variant of a single Enum that represents
//! all possible variants and states that a cell can take on.
//!
//! Every tick of the simulation, a function is evaluated that transforms a cell from its current state into the next
//! state.  Its only inputs are the cell itself and 2-dim array of its neighboring cells as `Option`s to account for
//! cases where the cell is on the edge of the universe.  The size of the the supplied array is dependant on the view
//! distance of the universe.

pub trait CellState {}

pub struct Cell<CellState> {
    state: CellState,
}
