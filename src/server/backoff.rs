use std::time::Duration;

pub struct Backoff {
    base: u64,
    exp: u32,
    max: u64,
}

impl Backoff {
    pub fn new(base_as_millis: u64, max_as_seconds: u64) -> Self {
        Self {
            base: base_as_millis,
            exp: 0,
            max: max_as_seconds * 1000,
        }
    }

    pub fn next(&mut self) -> Duration {
        let duration = self.base * (1 << self.exp);
        let delay = if duration >= self.max {
            self.max
        } else {
            self.exp += 1;
            duration
        };

        Duration::from_millis(delay)
    }

    pub fn reset(&mut self) {
        self.exp = 0;
    }
}
