pub mod collections;
pub mod endpoint;

use std::env;

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
