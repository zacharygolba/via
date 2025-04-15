#![allow(dead_code)]

use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Elapsed(u128, UnitOfTime);

pub fn fmt_time(duration: Duration) -> Elapsed {
    let micros = duration.as_micros();

    if micros < 1_000 {
        Elapsed(micros, UnitOfTime::Micros)
    } else {
        Elapsed(micros / 1_000, UnitOfTime::Millis)
    }
}

#[allow(dead_code)]
pub async fn timed<F: Future>(future: F) -> (F::Output, Duration) {
    let now = Instant::now();
    (future.await, now.elapsed())
}

#[derive(Debug)]
enum UnitOfTime {
    Micros,
    Millis,
}

impl Display for Elapsed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self(elapsed, UnitOfTime::Micros) => {
                write!(f, "{}µs", elapsed)
            }
            Self(elapsed, UnitOfTime::Millis) => {
                write!(f, "{}ms", elapsed)
            }
        }
    }
}
