#![feature(try_from)]

extern crate itertools;
extern crate ketos;
extern crate minutiae;
extern crate pcg;
extern crate rand;
extern crate uuid;

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use ketos::{Context, GlobalScope, Scope, Value};
use ketos::compile::compile;
use ketos::bytecode::Code;
use ketos::lexer::Lexer;
use ketos::parser::Parser;
use ketos::rc_vec::RcVec;
use ketos::restrict::RestrictConfig;
use itertools::Itertools;
use minutiae::prelude::*;
use minutiae::engine::serial::SerialEngine;
use minutiae::engine::iterator::SerialEntityIterator;
use minutiae::driver::middleware::MinDelay;
use minutiae::driver::BasicDriver;
use minutiae::universe::Universe2D;
use minutiae::util::{debug, translate_entity};
use pcg::PcgRng;
use rand::SeedableRng;
use uuid::Uuid;

#[cfg(feature = "wasm")]
extern {
    pub fn canvas_render(pixbuf_ptr: *const u8);
}

const UNIVERSE_SIZE: usize = 80;
const PRNG_SEED: [u64; 2] = [198918237842, 9];

const UNIVERSE_LENGTH: usize = UNIVERSE_SIZE * UNIVERSE_SIZE;

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

fn get_ant_global_scope() -> Scope {
    let global_scope = ketos::scope::GlobalScope::default("ant");
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum CellContents {
    Empty,
    Filled(u8),
    Food(u16),
    Anthill,
}

#[derive(Clone, Debug)]
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

impl Ant {
    pub fn from_source(src: &str) -> Result<Self, String> {
        let context = get_ant_default_context();
        let codes = get_codes_from_source(&context, src)?;

        Ok(Ant {
            code: codes,
            context: context,
            holding: CellContents::Empty,
        })
    }
}

impl Debug for Ant {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "Ant {{ code: {:?}, context: {{..}}, holding: {:?} }}", self.code, self.holding)
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

#[derive(Clone, Debug)]
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

}

impl CellAction<CS> for CA {}

#[derive(Debug)]
enum EA {

}

type U = Universe2D<CS, ES, MES>;

fn map_value_to_self_action(val: &Value) -> Result<SelfAction<CS, ES, EA>, String> {
    match val {
        &Value::List(ref list) => {
            if list.is_empty() {
                return Err("The provided action list was empty!".into());
            }

            println!("LIST: {:?}", list);

            match &list[0] {
                &Value::String(ref action_type) => match action_type.as_ref() {
                    "translate" => {
                        if list.len() != 3 {
                            return Err(format!("Invalid amount of arguments provided to translate action: {}", list.len() - 1));
                        }

                        let arg1: isize = match &list[1] {
                            &Value::Integer(ref int) => match int.to_isize() {
                                Some(i) => i,
                                None => {
                                    return Err(format!("Integer provided to argument 1 converted into `None`!"))
                                }
                            },
                            _ => {
                                return Err(format!(
                                    "Invalid arg type of {} provided to argument 1 of translate action!",
                                    list[1].type_name()
                                ));
                            },
                        };

                        let arg2: isize = match &list[2] {
                            &Value::Integer(ref int) => match int.to_isize() {
                                Some(i) => i,
                                None => {
                                    return Err(format!("Integer provided to argument 2 converted into `None`!"))
                                }
                            },
                            _ => {
                                return Err(format!(
                                    "Invalid arg type of {} provided to argument 2 of translate action!",
                                    list[2].type_name()
                                ));
                            },
                        };

                        let action = SelfAction::Translate(arg1, arg2);
                        println!("GENERATED TRANSLATE ACTION: {:?}", action);
                        Ok(action)
                    },
                    _ => Err(format!("Invalid action type of `{}` supplied!", action_type)),
                },
                _ => Err(format!("Invalid argument type of {} provided for action identifier!", list[0].type_name()))
            }
        },
        _ => Err(format!("Invalid value type of {} jammed into action buffer.", val.type_name()))
    }
}

fn map_value_to_cell_action(_val: &Value) -> Result<(CA, usize), String> {
    unimplemented!();
}

fn map_value_to_entity_action(_val: &Value) -> Result<(EA, usize, Uuid), String> {
    unimplemented!();
}

impl EntityAction<CS, ES> for EA {}

struct WorldGenerator;

impl Generator<CS, ES, MES> for WorldGenerator {
    fn gen(&mut self, _conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        let _rng = PcgRng::from_seed(PRNG_SEED);
        let cells = vec![Cell { state: CS::default() }; UNIVERSE_LENGTH];
        let mut entities = vec![Vec::new(); UNIVERSE_LENGTH];

        let ant_src = include_str!("./ant.lisp");
        let ant_entity: Entity<CS, ES, MES> = Entity::new(ES::from(Ant::from_source(ant_src).unwrap()), MES::default());
        entities[0] = vec![ant_entity];

        (cells, entities)
    }
}

fn reset_action_buffers(context: &Context, universe_index: usize) {
    let scope: &GlobalScope = context.scope();
    scope.add_named_value("__CELL_ACTIONS", Value::Unit);
    scope.add_named_value("__SELF_ACTIONS", Value::Unit);
    scope.add_named_value("__ENTITY_ACTIONS", Value::Unit);
    scope.add_named_value("UNIVERSE_INDEX", Value::Integer(ketos::integer::Integer::from_usize(universe_index)))
}

fn get_list_by_name(scope: &Scope, name: &str) -> Result<RcVec<Value>, String> {
    match scope.get_named_value(name) {
        Some(buf) => match buf {
            Value::List(list) => Ok(list),
            Value::Unit => Ok(RcVec::new(vec![])),
            _ => {
                return Err(format!("{} has been changed to an invalid type of {}!", name, buf.type_name()));
            },
        }
        None => {
            return Err(format!("The variable named {} was deleted!", name));
        },
    }
}

fn process_action_buffers(
    context: &Context,
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) -> Result<(), String> {
    let scope = context.scope();

    let cell_action_list = get_list_by_name(scope, "__CELL_ACTIONS")?;

    for val in &cell_action_list {
        let (action, universe_index): (CA, usize) = map_value_to_cell_action(val)?;
        cell_action_executor(action, universe_index);
    }

    let self_action_list = get_list_by_name(scope, "__SELF_ACTIONS")?;

    for val in &self_action_list {
        let action: SelfAction<CS, ES, EA> = map_value_to_self_action(val)?;
        self_action_executor(action);
    }

    let entity_action_list = get_list_by_name(scope, "__ENTITY_ACTIONS")?;

    for val in &entity_action_list {
        let (action, entity_index, uuid): (EA, usize, Uuid) = map_value_to_entity_action(val)?;
        entity_action_executor(action, entity_index, uuid);
    }

    Ok(())
}

struct AntEngine;

fn exec_cell_action(
    owned_action: &OwnedAction<CS, ES, CA, EA>,
    _cells: &mut [Cell<CS>],
    entities: &mut EntityContainer<CS, ES, MES>
) {
    let (_entity, _entity_universe_index) = match entities.get_verify_mut(owned_action.source_entity_index, owned_action.source_uuid) {
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

fn exec_self_action(
    universe: &mut U,
    action: &OwnedAction<CS, ES, CA, EA>
) {
    match action.action {
        Action::SelfAction(SelfAction::Translate(x_offset, y_offset)) => translate_entity(
            x_offset,
            y_offset,
            &mut universe.entities,
            action.source_entity_index,
            action.source_uuid,
            UNIVERSE_SIZE
        ),
        Action::EntityAction{ .. } | Action::CellAction{ .. } => unreachable!(),
         _ => unimplemented!(),
    }
}

fn exec_entity_action(_action: &OwnedAction<CS, ES, CA, EA>) {
    unimplemented!(); // TODO
}

impl SerialEngine<CS, ES, MES, CA, EA, SerialEntityIterator<CS, ES>, U> for AntEngine {
    fn iter_entities(&self, _universe: &U) -> SerialEntityIterator<CS, ES> {
        SerialEntityIterator::new(UNIVERSE_SIZE)
    }

    fn exec_actions(
        &self,
        universe: &mut U,
        cell_actions: &[OwnedAction<CS, ES, CA, EA>],
        self_actions: &[OwnedAction<CS, ES, CA, EA>],
        entity_actions: &[OwnedAction<CS, ES, CA, EA>]
    ) {
        for cell_action in cell_actions { exec_cell_action(cell_action, &mut universe.cells, &mut universe.entities); }
        for self_action in self_actions { exec_self_action(universe, self_action); }
        for entity_action in entity_actions { exec_entity_action(entity_action); }
    }

    fn drive_entity(
        &mut self,
        universe_index: usize,
        entity: &Entity<CS, ES, MES>,
        _: &U,
        cell_action_executor: &mut FnMut(CA, usize),
        self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
        entity_action_executor: &mut FnMut(EA, usize, Uuid)
    ) {
        match entity.state {
            ES::Ant(Ant { ref code, ref context, .. }) => {
                reset_action_buffers(context, universe_index);

                for c in code {
                    match ketos::exec::execute(context, Rc::clone(&c)) {
                        Ok(_) => (),
                        Err(err) => {
                            println!("Entity script errored: {:?}", err);
                            return;
                        },
                    };
                }

                match process_action_buffers(
                    context,
                    cell_action_executor,
                    self_action_executor,
                    entity_action_executor
                ) {
                    Ok(()) => (),
                    Err(err) => println!("Error while retrieving action buffers from context: {}", err),
                }

                println!("TICK");
            }
        }
    }
}

type OurSerialEngine = Box<SerialEngine<CS, ES, MES, CA, EA, SerialEntityIterator<CS, ES>, U>>;

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
            CellContents::Food(_) => [200, 30, 40, 255], // TODO: Different colors for different food amounts
            CellContents::Filled(_) => [230, 230, 230, 255],
        }
    }
}

#[cfg(feature = "wasm")]
fn init(
    universe: U,
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
    universe: U,
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
    let universe = Universe2D::new(conf, &mut WorldGenerator);
    let engine: OurSerialEngine = Box::new(AntEngine);

    init(universe, engine);
}
