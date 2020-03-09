use crate::{handler::DynMiddleware, http::Extensions, Middleware};
use std::sync::Arc;
use verbs::*;

pub(crate) type Router = radr::Router<Endpoint>;

pub trait Service: Send + Sync + 'static {
    fn mount(self: Arc<Self>, location: &mut Location);
}

pub struct Location<'a> {
    pub(crate) state: &'a mut Extensions,
    pub(crate) value: radr::Location<'a, Endpoint>,
}

#[derive(Default)]
pub(crate) struct Endpoint {
    pub(crate) verbs: Map<DynMiddleware>,
    pub(crate) stack: Vec<DynMiddleware>,
}

impl Endpoint {
    pub fn expose(&mut self, verb: Verb, handler: impl Middleware) -> &mut Self {
        self.verbs.insert(verb, Box::new(handler));
        self
    }

    pub fn middleware(&mut self, handler: impl Middleware) -> &mut Self {
        self.stack.push(Box::new(handler));
        self
    }
}

impl<'a> Location<'a> {
    #[inline]
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            state: self.state,
            value: self.value.at(path),
        }
    }

    #[inline]
    pub fn expose(&mut self, verb: Verb, handler: impl Middleware) {
        self.value.expose(verb, handler);
    }

    #[inline]
    pub fn middleware(&mut self, handler: impl Middleware) {
        self.value.middleware(handler);
    }

    #[inline]
    pub fn mount(&mut self, service: impl Service) {
        Arc::new(service).mount(self);
    }
}
