use rand::rngs::StdRng;
use rand::{thread_rng, Rng, SeedableRng};
use std::time::Duration;

use crate::Error;

pub struct Backoff {
    base: u32,
    exp: u32,
    max: u32,
    rng: StdRng,
}

fn millis_from_seconds(seconds: u32) -> Result<u32, Error> {
    seconds.checked_mul(1000).ok_or_else(|| {
        Error::new(format!(
            "unable to calculate milliseconds from ({} * 1000).",
            seconds
        ))
    })
}

impl Backoff {
    /// Creates a new `Backoff` with the provided base and max values.
    pub fn new(base_as_millis: u32, max_as_seconds: u32) -> Result<Self, Error> {
        let base = base_as_millis;
        let exp = 0;
        let max = millis_from_seconds(max_as_seconds)?;
        let rng = StdRng::from_rng(thread_rng())?;

        Ok(Self {
            base,
            exp,
            max,
            rng,
        })
    }

    /// Returns the next delay with jitter as a `Duration`.
    pub fn next(&mut self) -> Duration {
        // Calculate the delay based on the value of `self.base` and `self.exp`.
        let delay = Duration::from_millis(self.delay() as u64);
        // Calculate the factor used to add jitter to `delay` using a random f64
        // between 0.1 and 1 (inclusive).
        let factor = self.rng.gen_range(0.1..=1.0);

        // Add jitter to `delay`. This is used to provide some level of protection
        // against the thundering herd problem. In practice, it's best to take a
        // multi-headed approach to preventing thundering herds, starting at the
        // network layer of your tech stack.
        //
        // In the face of a sophisticated thundering herd attack, our goal is to
        // buy you and your ops team time to properly filter out the source of
        // malicious traffic.
        //
        // If you have an idea that can improve the resilience or security of Via
        // that fall within the bounds of what is expected of an application server,
        // create an issue or submit a pull request. Your contribution would be
        // appreciated.
        //
        delay.mul_f64(factor)
    }

    /// Resets the backoff to it's initial state.
    pub fn reset(&mut self) {
        self.exp = 0;
    }

    /// Calculates and returns the next delay as `self.base * 2^(self.exp)`. If
    /// the calculated delay is greater than or equal to `self.max`, `self.max`
    /// is returned instead.
    fn delay(&mut self) -> u32 {
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
