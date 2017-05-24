# Minutiae Remote Communication Library

Defines code shared between the client and server for the facilitation of communication between a light client and centralized server.  Since both ends are written in Rust, we can use the same code on both ends which really simplifies the communication process.

We can even get away with things as low-level as passing binary representations aroud over the WebSocket connection since it's going to be read back in as a Rust data structure anyway.
