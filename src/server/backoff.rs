use rand::Rng;
use std::time::Duration;

pub struct Backoff {
    base: u64,
    exp: u64,
    max: u64,
}

impl Backoff {
    /// Creates a new `Backoff` with the provided base and max values.
    pub fn new(base_as_millis: u64, max_as_seconds: u64) -> Self {
        Self {
            base: base_as_millis,
            exp: 1,
            max: max_as_seconds * 1000,
        }
    }

    /// Returns the next delay with jitter as a `Duration`.
    pub fn next(&mut self) -> Duration {
        // Calculate the delay based on the value of `self.base` and `self.exp`.
        let delay = self.delay();
        // Add jitter to `delay`. The jitter is a random value between 0 and
        // `delay`. This is used to provide some level of protection against
        // the thundering herd problem.
        let jitter = rand::thread_rng().gen_range(0..=delay);

        Duration::from_millis(jitter)
    }

    /// Resets the backoff to it's initial state.
    pub fn reset(&mut self) {
        self.exp = 1;
    }

    /// Calculates and returns the next delay as `self.base * 2^(self.exp)`. If
    /// the calculated delay is greater than or equal to `self.max`, `self.max`
    /// is returned instead.
    fn delay(&mut self) -> u64 {
        // Calculate the delay as base * 2^exp.
        let delay = self.base * (1 << self.exp);

        // Check if the delay is greater than or equal to `self.max`. If it
        // is, return early with `self.max` and do not increment `self.exp`.
        if delay >= self.max {
            return self.max;
        }

        // Increment `self.exp` for the next iteration.
        self.exp += 1;

        // Return the calculated delay.
        delay
    }
}
