use http::method::Method;
use std::ops::BitOr;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Verb(u16);

impl Verb {
    pub const CONNECT: Verb = Verb(0b0_0000_0001);
    pub const DELETE: Verb = Verb(0b0_0000_0010);
    pub const GET: Verb = Verb(0b0_0000_0100);
    pub const HEAD: Verb = Verb(0b0_0000_1000);
    pub const OPTIONS: Verb = Verb(0b0_0001_0000);
    pub const PATCH: Verb = Verb(0b0_0010_0000);
    pub const POST: Verb = Verb(0b0_0100_0000);
    pub const PUT: Verb = Verb(0b0_1000_0000);
    pub const TRACE: Verb = Verb(0b1_0000_0000);

    pub const fn all() -> Verb {
        Verb(0b1_1111_1111)
    }

    pub const fn none() -> Verb {
        Verb(0)
    }

    pub fn intersects(self, other: Verb) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Verb {
    type Output = Verb;

    fn bitor(self, other: Verb) -> Self::Output {
        Verb(self.0 | other.0)
    }
}

impl From<Method> for Verb {
    fn from(method: Method) -> Verb {
        Verb::from(&method)
    }
}

impl<'a> From<&'a Method> for Verb {
    fn from(method: &'a Method) -> Verb {
        match *method {
            Method::CONNECT => Verb::CONNECT,
            Method::DELETE => Verb::DELETE,
            Method::GET => Verb::GET,
            Method::HEAD => Verb::HEAD,
            Method::OPTIONS => Verb::OPTIONS,
            Method::PATCH => Verb::PATCH,
            Method::POST => Verb::POST,
            Method::PUT => Verb::PUT,
            Method::TRACE => Verb::TRACE,
            _ => Verb::none(),
        }
    }
}
