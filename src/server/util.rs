use std::fmt::{self, Display, Formatter};
use std::time::Duration;

#[derive(Debug)]
pub struct Elapsed(u128, UnitOfTime);

#[derive(Debug)]
enum UnitOfTime {
    Micros,
    Millis,
}

pub fn fmt_elapsed(duration: Duration) -> Elapsed {
    let micros = duration.as_micros();

    if micros < 1000 {
        Elapsed(micros, UnitOfTime::Micros)
    } else {
        Elapsed(micros / 1000, UnitOfTime::Millis)
    }
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
