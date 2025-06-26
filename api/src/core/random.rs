use axsync::Mutex;
use lazy_static::lazy_static;
use rand_mt::Mt64;

lazy_static! {
    /// A globally accessible random number generator.
    pub static ref RANDOM_GENERATOR: Mutex<Mt64> = {
        let seed = axhal::time::monotonic_time_nanos();
        Mutex::new(Mt64::new(seed))
    };
}

pub fn random_u64() -> u64 {
    let mut rng = RANDOM_GENERATOR.lock();
    rng.next_u64()
}

pub fn random_u32() -> u32 {
    let mut rng = RANDOM_GENERATOR.lock();
    rng.next_u32()
}
