use minutiae::prelude::*;

use super::*;

pub struct WorldGenerator;

impl Generator<CS, ES, MES> for WorldGenerator {
    fn gen(&mut self, conf: &UniverseConf) -> (Vec<Cell<CS>>, Vec<Vec<Entity<CS, ES, MES>>>) {
        unimplemented!();
    }
}
