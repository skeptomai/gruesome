
use rand::{RngCore,SeedableRng,Rng,rngs::StdRng};

/// RandMode controls random generator behaviour. May be predictable for testing or truly random for gameplay
pub enum RandMode {
    Predictable,
    RandomUniform,
}

pub struct ZRand {
    rng : Box<dyn RngCore>,
    rand_mode : RandMode,
}

impl ZRand {
    pub fn new(rm: RandMode) -> ZRand {
        ZRand { rng: Box::new(rand::thread_rng()), rand_mode: rm }
    }

    pub fn new_uniform() -> ZRand {
        ZRand::new(RandMode::RandomUniform)
    }


    pub fn new_predictable(seed: u64) -> ZRand {
        ZRand {rng: Box::new(StdRng::seed_from_u64(seed)), rand_mode: RandMode::Predictable}
    }

    /// gen_unsigned_rand generates unsigned in range [0..32767]
    pub fn gen_unsigned_rand(&mut self) -> u16 {
        // NOTE: This could probably be (u16::MAX +1) / 2
        self.rng.gen_range(0..32768)
    }
}
