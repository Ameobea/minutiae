//! Minutae simulation client.  See README.md for more information.

/// Holds the state of the universe as viewed the client.  Only holds a shallow view as calculated by the server.
pub struct Client<T> {
    pub cells: Vec<T>,
}

impl<T> Client<T> {
    pub fn new(universe_size: usize) -> Self {
        Client {
            cells: Vec::with_capacity(universe_size * universe_size),
        }
    }
}

fn main() {
    
}
