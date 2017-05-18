# Minutae
Minutae is a simulation engine that operates on a finite 2-dimensional universe populated by **cells** and **entities**.  It is designed to provide a modular system on top of which simulations of various types can be designed.

To view an example of a simulation implemented using this system and compiled to Asm.js using Emscripten, check out https://ameo.link/fish.html

# Application Overview
The Minutae engine is split into separate modules that can be combined in order to alter the way that simulations are executed.  The entire application is designed to make extensive use of the Rust type system, making it possible to use generic components in multiple different implementations.

The library itself has a very low set of dependencies, currently only making use of the `uuid` library and `num_cpus` for the parallel engine.  This makes it easy to compile the library down to a variety of targets; it has been implemented with great success in both asm.js and WebAssembly targets in the browser.

## Universe
The universe is the core structure that houses the simulation's state.  It contains a large vector containing all of the universe's cells and a container housing all of its entities.  The universe is always square has a set **size** which corresponds to its length/width (the number of cells/coordinates is `size^2`).

## Cells
Every grid of the universe is occupied by one cell.  Cells are static and do not take any action on their own.  They are acted upon by entities and the engine itself during simulation.  Cells are stateful and hold data that can be mutated each simulation tick.  The exact type of state that the cells hold is generic and is defined by the user's implementation; a good idea could be to implement their state as an enum.

## Entities
Entities also reside in a single coordinate of the universe but are handled separately from cells.  More than one entity can occupy the same coordinate.  Every tick, the universe's **Entity Driver** is invoked given an entity, that entity's state, and the universe's cells and entities.  The Entity may attempt any number of **Actions** in order to alter their own state or the state of the other cells/entities in the universe.

They hold two types of state: static state and mutable state.  The static state is controlled by the universe's engine and can't be directly modified by the entities.  The mutable state is directly accessible and writeable by the entities themselves during their simulation ticks but isn't accessible by other entities in the universe.

Each entity is assigned a unique UUID that can be used to target specific entities in entity actions.

## Actions
Actions are collected from every entity each tick and stored in a buffer, then applied together by the universe's `Engine`.  There are three types of actions: **Cell Actions**, **Self Actions**, and **Entity Actions**; each modifies the state of a different kind of object, as suggested by their names.  There are some pre-set implementations of these in the engine for things like translating entities and deleting them, but the majority of their implementation is left up to the user.

## Engine
The Engine is responsible for applying the actions returned by the entities.  Since it's possible for entities to attempt invalid actions or for two entities to experience conflicts, it's the engine's job to resolve them.

Two implementations of engine are provided with the library: a **Serial Engine** and a **Parallel Engine**.  They both function in basically the same way, but the parallel engine makes use of multiple threads for executing the entity driver on a universe's entities.

### Middleware
Middleware defines actions that are executed in between each simulation tick.  It provides full mutable access to the entire universe and can be used to implement a variety of functionality that takes place at a higher level of granularity than individual cells or entities provide.  A universe is constructed with any number of middleware objects that are applied one after another with the option of taking action either before or after the tick is applied.

## Performance
One of the main goals of this engine is to be highly efficient and keep overhead levels as low as possible.  Rust's zero-cost abstractions make it easy to allow for user-defined states and actions to be used across the entire engine while avoiding any runtime cost.

Unsafe code is used in the hottest parts of the engine, mostly with the purpose of avoiding bounds checks in situations where the size is statically known and enabling the parallel engine to work without a slew of lifetime-related issues.

# Contribution
I'm more than happy to work with people interested in this library in adapting it for their own purposes or adding additional features to suit their needs.  As always, if you wan to implement a feature or make a change, please open an issue so we can discuss it.