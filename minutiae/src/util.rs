//! General-purpose utility functions

use std::collections::HashMap;

use uuid::Uuid;

use cell::CellState;
    use entity::{Entity, EntityState, MutEntityState};

/// Given an index of the universe and the universe's size returns X and Y coordinates.
pub fn get_coords(index: usize, universe_size: usize) -> (usize, usize) {
    debug_assert!(index < universe_size * universe_size);
    let x = index % universe_size;
    let y = (index - x) / universe_size;
    (x, y)
}

/// Given an X and Y coordinate in the universe and the universe's size, returns the index of that coordinate in the universe.
pub fn get_index(x: usize, y: usize, universe_size: usize) -> usize {
    debug_assert!(x < universe_size);
    debug_assert!(y < universe_size);
    y * universe_size + x
}

/// Calculates the manhattan distance between the two provided grid cells
pub fn manhattan_distance(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let x_diff = if x1 < x2 { x2 - x2 } else { x1 - x2 };
    let y_diff = if y1 < y2 { y2 - y1 } else { y1 - y2 };
    x_diff + y_diff
}

/// Calculates the offset between two coordinates; where point 2 is located relative to point 1
pub fn calc_offset(x1: usize, y1: usize, x2: usize, y2: usize) -> (isize, isize) {
    (x2 as isize - x1 as isize, y2 as isize - y1 as isize)
}

/// Searches one coordinate of the universe and attempts to find the entity ID of the entity with the supplied UUID.
pub fn locate_entity_simple<C: CellState, E: EntityState<C>, M: MutEntityState>(
    uuid: Uuid, entities: &[Entity<C, E, M>]
) -> Option<usize> {
    entities.iter().position(|& ref entity| entity.uuid == uuid)
}

pub enum EntityLocation {
    Deleted, // entity no longer exists
    Expected(usize), // entity is where it's expecte to be with the returned entity index
    Moved(usize, usize, usize), // entity moved to (arg 0, arg 1) with entity index arg 2
}

/// Attempts to find the entity index of an entity with a specific UUID and a set of coordinates where it is expected to
/// be located.
pub fn locate_entity<C: CellState, E: EntityState<C>, M: MutEntityState>(
    entities: &[Vec<Entity<C, E, M>>], uuid: Uuid, expected_index: usize, entity_meta: &HashMap<Uuid, (usize, usize)>,
    universe_size: usize,
) -> EntityLocation {
    debug_assert!(expected_index < (universe_size * universe_size));
    // first attempt to find the entity at its expected coordinates
    match locate_entity_simple(uuid, &entities[expected_index]) {
        Some(entity_index) => EntityLocation::Expected(entity_index),
        None => {
            // unable to locate entity at its expected coordinates, so check the coordinates in the meta `HashMap`
            match entity_meta.get(&uuid) {
                Some(&(real_x, real_y)) => {
                    let real_index = get_index(real_x, real_y, universe_size);
                    let entity_index = locate_entity_simple(uuid, &entities[real_index])
                        .expect("Entity not present at coordinates listed in meta `HashMap`!");

                    EntityLocation::Moved(real_x, real_y, entity_index)
                },
                // If no entry in the `HashMap`, then the entity has been deleted.
                None => EntityLocation::Deleted,
            }
        }
    }
}

struct VisibleIterator {
    min_x: usize,
    max_x: usize,
    max_y: usize,
    cur_x: usize,
    cur_y: usize,
    first: bool,
}

// TODO: Optimize this.  The `if self.first` branch is one of the most expensive lines of code in this whole app
//       there's probably a way to make this work without any branches at all tbh.
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
    debug_assert!(cur_x < universe_size);
    debug_assert!(cur_y < universe_size);
    // both minimums and maximums are inclusive
    let min_y = if cur_y >= view_distance { cur_y - view_distance } else { 0 };
    let min_x = if cur_x >= view_distance { cur_x - view_distance } else { 0 };
    let max_y = if cur_y + view_distance < universe_size { cur_y + view_distance } else { universe_size - 1 };
    let max_x = if cur_x + view_distance < universe_size { cur_x + view_distance } else { universe_size - 1 };

    VisibleIterator {
        min_x: min_x,
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
