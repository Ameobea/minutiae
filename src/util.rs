//! General-purpose utility functions

use std::iter::Chain;
use itertools::chain;

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

/// Calculates the manhattan distance between the two provided grid cells
pub fn manhattan_distance(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let x_diff = if x1 < x2 { x2 - x2 } else { x1 - x2 };
    let y_diff = if y1 < y2 { y2 - y1 } else { y1 - y2 };
    x_diff + y_diff
}

struct VisibleIterator {
    universe_size: usize,
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
    cur_x: usize,
    cur_y: usize,
    first: bool,
}

impl Iterator for VisibleIterator {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            Some((self.cur_x, self.cur_y,))
        } else {
            if self.cur_x < self.max_x {
                self.cur_x += 1;
                Some((self.cur_x, self.cur_y,))
            } else {
                if self.cur_y < self.max_y {
                    self.cur_y += 1;
                    self.cur_x = self.min_x;
                    Some((self.cur_x, self.cur_y,))
                } else {
                    None
                }
            }
        }
    }
}

/// Given current X and Y coordinates of an entity and the view distance of the universe, creates an iterator visiting
/// the indexes of all visible grid coordinates.  Note that this will include the index of the source entity.
pub fn iter_visible(cur_x: usize, cur_y: usize, view_distance: usize, universe_size: usize) -> impl Iterator<Item=(usize, usize)> {
    // both minimums and maximums are inclusive
    let min_y = if cur_y >= view_distance { cur_y - view_distance } else { 0 };
    let min_x = if cur_x >= view_distance { cur_x - view_distance } else { 0 };
    let max_y = if cur_y + view_distance < universe_size { cur_y + view_distance } else { universe_size - 1 };
    let max_x = if cur_x + view_distance < universe_size { cur_x + view_distance } else { universe_size - 1 };

    VisibleIterator {
        universe_size: universe_size,
        min_x: min_x,
        min_y: min_y,
        max_x: max_x,
        max_y: max_y,
        cur_x: min_x,
        cur_y: min_y,
        first: true,
    }
}

#[test]
fn iter_visible_functionality() {
    println!("test");
    let universe_size = 50;
    let mut view_distance = 3;
    let mut cur_x = 6;
    let mut cur_y = 6;

    let indexes: Vec<(usize, usize)> = iter_visible(cur_x, cur_y, view_distance, universe_size).collect();
    assert!(indexes.len() == 49);

    view_distance = 4;
    cur_x = 3;
    cur_y = 2;
    let indexes: Vec<(usize, usize)> = iter_visible(cur_x, cur_y, view_distance, universe_size).collect();

    assert!(indexes.len() == 56);
}

#[test]
fn manhattan_distance_accuracy() {
    let x1 = 1;
    let y1 = 5;
    let x2 = 4;
    let y2 = 0;

    assert_eq!(manhattan_distance(x1, y1, x2, y2), 8);
}
