pub mod minheap;
pub mod pid;

use std::env;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

pub type HashMap<K, V> = fxhash::FxHashMap<K, V>;
pub type HashSet<K> = fxhash::FxHashSet<K>;

pub fn mk_rng() -> impl Rng {
    Pcg32::seed_from_u64(
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

pub fn duration_seconds(duration: &Duration) -> f64 {
    let nanos: f64 = duration.subsec_nanos().into();
    (duration.as_secs() as f64) + nanos / 1e9
}

pub fn seconds_duration(seconds: f64) -> Duration {
    assert!(seconds >= 0.0);
    let secs = seconds.trunc();
    let nanos = seconds.fract() * 1e9;
    Duration::new(secs as u64, nanos as u32)
}

pub fn find_duplicate<T>(items: &[T]) -> Option<(usize, usize)>
where
    T: PartialEq,
{
    let len = items.len();
    for i in 0..len {
        for j in (i + 1)..len {
            if items[i] == items[j] {
                return Some((i, j));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_duplicate() {
        let a1 = &[1, 2, 3, 1];
        assert_eq!(find_duplicate(a1), Some((0, 3)));

        let a2 = &[1, 2, 2, 1];
        assert_eq!(find_duplicate(a2), Some((0, 3)));

        let a3 = &[1, 1, 1, 1];
        assert_eq!(find_duplicate(a3), Some((0, 1)));

        let a4 = &[1, 2, 3, 4];
        assert_eq!(find_duplicate(a4), None);
    }
}
