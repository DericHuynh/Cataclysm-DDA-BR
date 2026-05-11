use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

/// A seeded deterministic RNG for reproducible tests and replays.
///
/// The simulation tick receives `&mut Rng` so that all randomness flows
/// through a single seeded source. This makes replays, crash reproduction,
/// and potential network synchronization tractable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeededRng {
    #[serde(skip, default = "SeededRng::default_rng")]
    inner: StdRng,
    /// The seed used to initialize this RNG (for serialization/debugging).
    seed: u64,
}

impl SeededRng {
    /// Create a new RNG from the given seed.
    pub fn new(seed: u64) -> Self {
        SeededRng {
            inner: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    /// Get the seed value (useful for reproducing a specific run).
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Default RNG for deserialization (uses seed=0).
    fn default_rng() -> StdRng {
        StdRng::seed_from_u64(0)
    }

    /// Generate a random boolean.
    pub fn gen_bool(&mut self, p: f64) -> bool {
        self.inner.random_bool(p)
    }

    /// Generate a random integer in [min, max] inclusive.
    pub fn gen_range(&mut self, min: u32, max: u32) -> u32 {
        self.inner.random_range(min..=max)
    }

    /// Generate a random f64 in [0.0, 1.0).
    pub fn gen_f64(&mut self) -> f64 {
        self.inner.random()
    }
}
