pub mod collections;
pub mod endpoint;
pub mod pid;

use std::env;
use std::time::{Duration, Instant};

use rand::prng::isaac::IsaacRng;
use rand::Rng;

pub fn mk_rng() -> impl Rng {
    IsaacRng::new_from_u64(
        env::var("PUDDLE_SEED")
            .map(|seed| {
                seed.parse()
                    .expect("Couldn't parse the seed into a number!")
            })
            .unwrap_or(0),
    )
}

pub struct Timer {
    pub start: Instant,
    pub lap: Instant,
}

impl Default for Timer {
    fn default() -> Self {
        Timer {
            start: Instant::now(),
            lap: Instant::now(),
        }
    }
}

impl Timer {
    pub fn new() -> Self {
        Timer::default()
    }

    pub fn lap(&mut self) -> Duration {
        self.lap_from_time(Instant::now())
    }

    pub fn lap_from_time(&mut self, now: Instant) -> Duration {
        let lap_time = now - self.lap;
        self.lap = now;
        lap_time
    }
}
