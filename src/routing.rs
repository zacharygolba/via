use crate::{handler::DynHandler, Context, Future, Handler, Next};
use std::ops::{Deref, DerefMut};
use verbs::{iter::Every, Map};

pub use verbs::Verb;

pub(crate) type Router = radr::Router<Endpoint>;

pub trait Mount {
    fn into(self, target: &mut Location);
}

#[derive(Default)]
pub struct Endpoint {
    verbs: Map<DynHandler>,
    stack: Vec<DynHandler>,
}

pub struct Location<'a> {
    pub(crate) value: radr::Location<'a, Endpoint>,
}

pub(crate) fn visit(router: &Router, mut context: Context) -> Future {
    let (parameters, method, path) = context.locate();
    let matches = router.visit(path).flat_map(|matched| {
        match matched.param {
            Some(("", _)) | Some((_, "")) | None => {}
            Some((name, value)) => {
                parameters.insert(name, value.to_owned());
            }
        }

        matched.stack.iter().chain(if matched.exact {
            matched.verbs.every(method)
        } else {
            Every::empty()
        })
    });

    Next::new(matches).call(context)
}

impl Endpoint {
    pub fn expose(&mut self, verb: Verb, handler: impl Handler) -> &mut Self {
        self.verbs.insert(verb, Box::new(handler));
        self
    }

    pub fn plug(&mut self, handler: impl Handler) -> &mut Self {
        self.stack.push(Box::new(handler));
        self
    }
}

impl<'a> Location<'a> {
    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            value: self.value.at(path),
        }
    }

    #[inline]
    pub fn mount(&mut self, mount: impl Mount) {
        mount.into(self);
    }
}

impl<'a> Deref for Location<'a> {
    type Target = Endpoint;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a> DerefMut for Location<'a> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
