//! Copied from https://github.com/Ameobea/noise-asmjs/blob/master/engine/src/main.rs

use std::os::raw::c_void;

use minutiae::prelude::*;
use noise::*;

use interop::GenType;
use super::*;

/// Holds the noise generator's state.  A pointer to this is passed along with all configuraiton functions.
pub struct NoiseEngine {
    pub generator_type: GenType,
    pub canvas_size: usize,
    pub seed: usize,
    pub octaves: usize,
    pub frequency: f32,
    pub lacunarity: f32,
    pub persistence: f32,
    pub zoom: f32,
    pub speed: f32,
    pub attenuation: f32,
    pub range_function: RangeFunction,
    pub enable_range: u32,
    pub displacement: f32,
    pub needs_update: bool, // flag indicating whether or not there are new stettings that need to be applied
    pub needs_resize: bool, // flag indicating if the universe itself needs to be resized or not
    pub needs_new_noise_gen: bool, // the type of noise generator itself needs to be changed
}

impl Default for NoiseEngine {
    fn default() -> Self {
        NoiseEngine {
            generator_type: GenType::Fbm,
            canvas_size: 0,
            seed: 101269420,
            octaves: 6,
            frequency: 1.0,
            lacunarity: 2.0,
            persistence: 0.5,
            speed: 0.00758,
            zoom: 0.0132312,
            attenuation: 2.0,
            enable_range: 0,
            displacement: 1.0,
            range_function: RangeFunction::Euclidean,
            needs_update: false,
            needs_resize: false,
            needs_new_noise_gen: false,
        }
    }
}

/// given a buffer containing all of the cells in the universe, calculates values for each of them using
/// perlin noise and sets their states according to the result.
pub fn drive_noise(cells_buf: &mut [Cell<CS>], seq: usize, noise: &NoiseModule<Point3<f32>, f32>, universe_size: usize, zoom: f32, speed: f32) {
    let fseq = seq as f32;
    for y in 0..universe_size {
        for x in 0..universe_size {
            // calculate noise value for current coordinate and sequence number
            let val1 = noise.get([x as f32 * zoom, y as f32 * zoom, fseq * speed]);
            let val2 = noise.get([y as f32 * zoom * 2., x as f32 * zoom * 2., fseq * speed]);

            // set the cell's state equal to that value
            let index = get_index(x, y, universe_size);
            cells_buf[index].state.noise_val_1 = val1;
            cells_buf[index].state.noise_val_2 = val2;
        }
    }
}

/// Given the ID of a noise engine, allocates an instance of it on the heap and returns a void reference to it.
/// Since `MultiFractal` can't be made into a trait object, this is the best optionsdfsfsdfsdfs
pub fn create_noise_engine(id: GenType) -> *mut c_void {
    match id {
        GenType::Fbm => Box::into_raw(Box::new(Fbm::new() as Fbm<f32>)) as *mut c_void,
        GenType::Worley => Box::into_raw(Box::new(Worley::new() as Worley<f32>)) as *mut c_void,
        GenType::OpenSimplex => Box::into_raw(Box::new(OpenSimplex::new())) as *mut c_void,
        GenType::Billow => Box::into_raw(Box::new(Billow::new() as Billow<f32>)) as *mut c_void,
        GenType::HybridMulti => Box::into_raw(Box::new(HybridMulti::new() as HybridMulti<f32>)) as *mut c_void,
        GenType::SuperSimplex => Box::into_raw(Box::new(SuperSimplex::new())) as *mut c_void,
        GenType::Value => Box::into_raw(Box::new(Value::new())) as *mut c_void,
        GenType::RidgedMulti => Box::into_raw(Box::new(RidgedMulti::new() as RidgedMulti<f32>)) as *mut c_void,
        GenType::BasicMulti => Box::into_raw(Box::new(BasicMulti::new() as BasicMulti<f32>)) as *mut c_void,
    }
}

/// Given a pointer to a noise engine of variable type and a settings struct, applies those settings based
/// on the capabilities of that noise modules.  For example, if the noise module doesn't implement `Seedable`,
/// the `seed` setting is ignored.
unsafe fn apply_settings(engine_conf: &NoiseEngine, engine: *mut c_void) -> *mut c_void {
    match engine_conf.generator_type {
        GenType::Fbm => {
            let gen = Box::from_raw(engine as *mut Fbm<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_octaves(engine_conf.octaves as usize);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_lacunarity(engine_conf.lacunarity);
            let gen = gen.set_persistence(engine_conf.persistence);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::Worley => {
            let gen = Box::from_raw(engine as *mut Worley<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_range_function(engine_conf.range_function.into());
            let gen = gen.enable_range(engine_conf.enable_range != 0);
            let gen = gen.set_displacement(engine_conf.displacement);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::OpenSimplex => {
            let gen = Box::from_raw(engine as *mut OpenSimplex);
            let gen = gen.set_seed(engine_conf.seed as u32);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::Billow => {
            let gen = Box::from_raw(engine as *mut Billow<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_octaves(engine_conf.octaves as usize);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_lacunarity(engine_conf.lacunarity);
            let gen = gen.set_persistence(engine_conf.persistence);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::HybridMulti => {
            let gen = Box::from_raw(engine as *mut HybridMulti<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_octaves(engine_conf.octaves as usize);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_lacunarity(engine_conf.lacunarity);
            let gen = gen.set_persistence(engine_conf.persistence);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::SuperSimplex => {
            let gen = Box::from_raw(engine as *mut SuperSimplex);
            let gen = gen.set_seed(engine_conf.seed as u32);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::Value => {
            let gen = Box::from_raw(engine as *mut Value);
            let gen = gen.set_seed(engine_conf.seed as u32);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::RidgedMulti => {
            let gen = Box::from_raw(engine as *mut RidgedMulti<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_octaves(engine_conf.octaves as usize);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_lacunarity(engine_conf.lacunarity);
            let gen = gen.set_persistence(engine_conf.persistence);
            let gen = gen.set_attenuation(engine_conf.attenuation);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
        GenType::BasicMulti => {
            let gen = Box::from_raw(engine as *mut BasicMulti<f32>);
            let gen = gen.set_seed(engine_conf.seed as u32);
            let gen = gen.set_octaves(engine_conf.octaves as usize);
            let gen = gen.set_frequency(engine_conf.frequency);
            let gen = gen.set_lacunarity(engine_conf.lacunarity);
            let gen = gen.set_persistence(engine_conf.persistence);
            Box::into_raw(Box::new(gen)) as *mut c_void
        },
    }
}

/// Defines a middleware that sets the cell state of all cells in the universe in accordance with a noise function.
pub struct NoiseMiddleware {
    pub conf: Box<NoiseEngine>,
    pub noise_engine: *mut c_void,
}

impl Middleware<
    CS, ES, MES, CA, EA, Box<SerialEngine<CS, ES, MES, CA, EA, SerialGridIterator, SerialEntityIterator<CS, ES>>>
> for NoiseMiddleware {
    fn after_render(&mut self, universe: &mut Universe<CS, ES, MES, CA, EA>) {
        // handle any new setting changes before rendering
        if self.conf.needs_update {
            // if self.conf.needs_resize {
            //     // resize the universe if the canvas size changed, matching that size.
            //     resize_universe(universe, self.conf.canvas_size);
            //     self.conf.needs_resize = false;
            // }

            if self.conf.needs_new_noise_gen {
                self.noise_engine = create_noise_engine(self.conf.generator_type);
                self.conf.needs_new_noise_gen = false;
            }

            // re-apply all settings to the noise module
            self.noise_engine = unsafe { apply_settings(&*self.conf, self.noise_engine) };

            self.conf.needs_update = false;
        }

        let module = match self.conf.generator_type {
            GenType::Fbm => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut Fbm<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::Worley => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut Worley<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::OpenSimplex => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut OpenSimplex) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::Billow => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut Billow<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::HybridMulti => drive_noise(&mut universe.cells, universe.seq, &unsafe { &*(self.noise_engine as *mut HybridMulti<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::SuperSimplex => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut SuperSimplex) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::Value => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut Value) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::RidgedMulti => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut RidgedMulti<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
            GenType::BasicMulti => drive_noise(&mut universe.cells, universe.seq, unsafe { &*(self.noise_engine as *mut BasicMulti<f32>) }, self.conf.canvas_size, self.conf.zoom, self.conf.speed),
        };
    }
}
