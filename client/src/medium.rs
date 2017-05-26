//! Defines a partial client that receives events from the server that don't directly correspond to pixel colors.
//! This is useful for simulations that have highly abstractable actions that affect multiple pixels.  It requires that
//! the client maintains a full copy of the universe's state including cell and entity states.

pub struct MediumClient {} // TODO
