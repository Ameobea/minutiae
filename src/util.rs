//! General-purpose utility functions

/// Given an index of the universe and the universe's size returns X and Y coordinates.
pub fn get_coords(index: usize, size: usize) -> (usize, usize) {
    let x = index % size;
    let y = (index - x) / size;
    (x, y)
}

/// Given an X and Y coordinate in the universe and the universe's size, returns the index of that coordinate in the universe.
pub fn get_index(x: usize, y: usize, size: usize) -> usize {
    y * size + x
}

// /// Moves an entity from one coordinate of the universe to another.
// fn move_entity<()
