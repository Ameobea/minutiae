use minutiae::prelude::*;
use uuid::Uuid;

use super::*;

pub fn our_entity_driver(
    source_universe_index: usize,
    entity: &Entity<CS, ES, MES>,
    entities: &EntityContainer<CS, ES, MES, usize>,
    cells: &[Cell<CS>],
    cell_action_executor: &mut FnMut(CA, usize),
    self_action_executor: &mut FnMut(SelfAction<CS, ES, EA>),
    entity_action_executor: &mut FnMut(EA, usize, Uuid)
) {

}
