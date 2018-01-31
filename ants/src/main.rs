#![feature(try_from)]

extern crate itertools;
extern crate ketos;
#[macro_use]
extern crate ketos_derive;
extern crate minutiae;
extern crate pcg;
extern crate rand;
extern crate uuid;

use std::convert::TryFrom;
use std::rc::Rc;

use ketos::{Context, GlobalScope, Scope, Value};
use ketos::compile::compile;
use ketos::bytecode::Code;
use ketos::lexer::Lexer;
use ketos::parser::Parser;
use ketos::restrict::RestrictConfig;
use ketos::structs::Struct;
use itertools::Itertools;
use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::SerialEntityIterator;
use minutiae::driver::middleware::MinDelay;
use minutiae::driver::BasicDriver;
use minutiae::util::debug;
use pcg::PcgRng;
use rand::{Rng, SeedableRng};
use uuid::Uuid;

#[cfg(feature = "wasm")]
extern {
    pub fn canvas_render(pixbuf_ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 800;
const ANT_COUNT: usize = 17;
const FOOD_DEPOSIT_COUNT: usize = 25;
const FOOD_DEPOSIT_SIZE: usize = 76;
const FOOD_DEPOSIT_RADIUS: usize = 8;
const MAX_FOOD_QUANTITY: u16 = 4000;
const PRNG_SEED: [u64; 2] = [198918237842, 9];
const ANT_FOOD_CAPACITY: usize = 12;

const UNIVERSE_LENGTH: usize = UNIVERSE_SIZE * UNIVERSE_SIZE;

#[derive(Clone, Copy, PartialEq)]
enum CellContents {
    Empty,
    Filled(u8),
    Food(u16),
    Anthill,
}

#[derive(Clone)]
struct CS {
    contents: CellContents,
}

impl CellState for CS {}

impl Default for CS {
    fn default() -> Self {
        CS { contents: CellContents::Empty }
    }
}

#[derive(Clone)]
struct Ant {
    code: Vec<Rc<Code>>,
    context: Context,
    holding: CellContents,
}

fn get_codes_from_source(context: &Context, src: &str) -> Result<Vec<Rc<Code>>, String> {
    let lexer = Lexer::new(src, 0);
    Parser::new(&context, lexer)
        .parse_exprs()
        .map_err(debug)?
        .iter()
        .map(|v| {
            println!("VALUE: {:?}", v);
            compile(&context, v)
        })
        .fold_results(Vec::new(), |mut acc, code| {
            acc.push(Rc::new(code));
            acc
        })
        .map_err(debug)
}

fn get_ant_restrictions() -> RestrictConfig {
    RestrictConfig::strict()
}

fn foreign_fn(context: &Context, values: &mut [Value]) -> Result<Value, ketos::Error> {
    println!("CALLED FOREIGN FUNCTION: {:?}", values);
    Ok(Value::Unit)
}

fn get_ant_global_scope() -> Scope {
    let global_scope = ketos::scope::GlobalScope::default("ant");
    global_scope.add_value_with_name("printer", |name| Value::new_foreign_fn(name, foreign_fn));
    global_scope.add_named_value("UNIVERSE_SIZE", UNIVERSE_SIZE.into());
    return Rc::new(global_scope)
}

fn get_ant_default_context() -> ketos::Context {
    let scope = get_ant_global_scope();
    let restrictions = get_ant_restrictions();
    let context = ketos::Context::new(scope, restrictions);

    // Fill the context with default items from our "standard library"
    let std_src = include_str!("./ant_std.lisp");
    let codes: Vec<Rc<Code>> = get_codes_from_source(&context, std_src)
        .expect("You've got syntax errors in your standard library!");

    for code in &codes {
        ketos::exec::execute(&context, Rc::clone(code))
            .expect("Error while executing standard library code!");
    }

    context
}

impl Ant {
    pub fn from_source(src: &str) -> Result<Self, String> {
        let context = get_ant_default_context();
        let codes = get_codes_from_source(&context, src)?;

        Ok(Ant {
            code: codes,
            context: get_ant_default_context(),
            holding: CellContents::Empty,
        })
    }
}

impl<'a> From<&'a ES> for Option<&'a Ant> {
    fn from(entity_state: &'a ES) -> Self {
        match entity_state {
            &ES::Ant(ref ant) => Some(ant),
        }
    }
}

impl<'a> From<&'a mut ES> for Option<&'a mut Ant> {
    fn from(entity_state: &'a mut ES) -> Self {
        match entity_state {
            &mut ES::Ant(ref mut ant) => Some(ant),
        }
    }
}

#[derive(Clone)]
enum ES {
    Ant(Ant),
}

impl EntityState<CS> for ES {}

impl From<Ant> for ES {
    fn from(ant: Ant) -> Self {
        ES::Ant(ant)
    }
}

#[derive(Clone)]
struct MES(ketos::Value);

impl Default for MES {
    fn default() -> Self {
        MES(ketos::Value::Unit)
    }
}

impl MutEntityState for MES {}

enum CA {
    LaySearchPheremone, // Deposit a pheremone on the current coordinate indicating that we were here while searching for food
    LayFoundPheremone, // Deposit a pheremone on the current coordinate indicating that we're returning with food
    CollectFood(usize), // Collects some food from the specified universe index
}

impl CellAction<CS> for CA {}

enum EA {

}

impl TryFrom<Value> for EA {
    type Error = String;

    fn try_from(val: Value) -> Result<Self, String> {
        match val {
            Value::Struct(_struct) => EA::try_from(*_struct),
            _ => Err(format!("Invalid value type of {} jammed into action buffer.", val.type_name()))
        }
    }
}

impl TryFrom<(Rc<NameStore>, Struct)> for EA {
    type Error = String;

    fn try_from((name_store, _struct_: (Struct)) -> Result<Self, String> {
        match _struct.def().name() {

        }
    }
}

impl EntityAction<CS, ES> for EA {}

struct WorldGenerator;

/// Given a coordinate, selects a point that's less than `size` units away from the source coordinate as calculated using
/// Manhattan distance.  The returned coordinate is guarenteed to be valid and within the universe.
fn rand_coord_near(rng: &mut PcgRng, src_index: usize, max_distance: usize) -> usize {
    let distance = rng.gen_range(0, max_distance + 1) as isize;
    loop {
        let x_mag = rng.gen_range(0, distance);
        let y_mag = distance - x_mag;

        let (x_offset, y_offset) = match rng.gen_range(0, 3) {
            0 => (x_mag, y_mag),
            1 => (-x_mag, y_mag),
            2 => (x_mag, -y_mag),
            3 => (-x_mag, -y_mag),
            _ => unreachable!(),
        };

        let (x, y) = get_coords(src_index, UNIVERSE_SIZE);
        let dst_x = x as isize + x_offset;
        let dst_y = y as isize + y_offset;
        if dst_x >= 0 && dst_x < UNIVERSE_SIZE as isize && dst_y >= 0 && dst_y < UNIVERSE_SIZE as isize {
            return get_index(dst_x as usize, dst_y as usize, UNIVERSE_SIZE);
        }
    }
}

impl Generator<CS, ES, MES, CA, EA> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        let mut rng = PcgRng::from_seed(PRNG_SEED);
        let mut cells = vec![Cell { state: CS::default() }; UNIVERSE_LENGTH];
        let mut entities = vec![Vec::new(); UNIVERSE_LENGTH];

        let ant_src = include_str!("./ant.lisp");
        let ant_entity: Entity<CS, ES, MES> = Entity::new(ES::from(Ant::from_source(ant_src).unwrap()), MES::default());
        entities[0] = vec![ant_entity];

        (cells, entities)
    }
}

fn reset_action_buffers(context: &Context) {
    let scope: &GlobalScope = context.scope();
    scope.add_named_value("__CELL_ACTIONS", Value::Unit);
    scope.add_named_value("__SELF_ACTIONS", Value::Unit);
    scope.add_named_value("__ENTITY_ACTIONS", Value::Unit);
}

fn entity_driver(
    universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {
    match entity.state {
        ES::Ant(Ant { ref code, ref context, holding }) => {
            for c in code {
                ketos::exec::execute(context, Rc::clone(c)).expect("Ant code broken.");
            }
        }
    }
}

struct AntEngine;

fn exec_cell_action(
    owned_action: &OwnedAction<CS, ES, CA, EA>,
    cells: &mut [Cell<CS>],
    entities: &mut EntityContainer<CS, ES, MES>
) {
    let (entity, entity_universe_index) = match entities.get_verify_mut(owned_action.source_entity_index, owned_action.source_uuid) {
        Some((entity, universe_index)) => (entity, universe_index),
        None => { return; }, // The entity been deleted, so abort.
    };

    match &owned_action.action {
        &Action::CellAction {ref action, ..} => match action {
            _ => unimplemented!(),
        },
        _ => unreachable!(),
    }
}

fn exec_self_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

fn exec_entity_action(action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

impl SerialEngine<CS, ES, MES, CA, EA, SerialEntityIterator<CS, ES>> for AntEngine {
    fn iter_entities(&self, entities: &[Vec<Entity<CS, ES, MES>>]) -> SerialEntityIterator<CS, ES> {
        SerialEntityIterator::new(UNIVERSE_SIZE)
    }

    fn exec_actions(
        &self, universe: &mut Universe<CS, ES, MES, CA, EA>, cell_actions: &[OwnedAction<CS, ES, CA, EA>],
        self_actions: &[OwnedAction<CS, ES, CA, EA>], entity_actions: &[OwnedAction<CS, ES, CA, EA>]
    ) {
        for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe.cells, &mut universe.entities); }
        for self_action in self_actions { exec_self_action(self_action); }
        for entity_action in entity_actions { exec_entity_action(entity_action); }
    }
}

type OurSerialEngine = Box<SerialEngine<CS, ES, MES, CA, EA, SerialEntityIterator<CS, ES>>>;

/// Given a coordinate of the universe, uses state of its cell and the entities that reside in it to determine a color
/// to display on the canvas.  This is called each tick.  The returned value is the color in RGBA.
fn calc_color(
    cell: &Cell<CS>,
    entity_indexes: &[usize],
    entity_container: &EntityContainer<CS, ES, MES>
) -> [u8; 4] {
    if !entity_indexes.is_empty() {
        for i in entity_indexes {
            match unsafe { &entity_container.get(*i).state } {
                &ES::Ant { .. } => { return [91, 75, 11, 255] },
            }
        }
        [12, 24, 222, 255]
    } else {
        match cell.state.contents {
            CellContents::Anthill => [222, 233, 244, 255],
            CellContents::Empty => [12, 12, 12, 255],
            CellContents::Food(amount) => [200, 30, 40, 255], // TODO: Different colors for different food amounts
            CellContents::Filled(_) => [230, 230, 230, 255],
        }
    }
}

#[cfg(feature = "wasm")]
fn init(
    universe: Universe<CS, ES, MES, CA, EA>,
    engine: OurSerialEngine
) {
    use minutiae::emscripten::{EmscriptenDriver, CanvasRenderer};

    let driver = EmscriptenDriver;

    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(CanvasRenderer::new(UNIVERSE_SIZE, calc_color, canvas_render)),
    ]);
}

#[cfg(not(feature = "wasm"))]
fn init(
    universe: Universe<CS, ES, MES, CA, EA>,
    engine: OurSerialEngine
) {
    let driver = BasicDriver;

    driver.init(universe, engine, &mut [
        Box::new(MinDelay::from_tps(59.99)),
        Box::new(minutiae::driver::middleware::gif_renderer::GifRenderer::new(
            "./out.gif", UNIVERSE_SIZE, calc_color
        )),
    ]);
}

fn main() {
    let conf = UniverseConf {
        size: 800,
        view_distance: 1,
    };
    let universe = Universe::new(conf, &mut WorldGenerator, cell_mutator, entity_driver);
    let engine: OurSerialEngine = Box::new(AntEngine);

    init(universe, engine);
}
