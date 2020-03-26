use crate::middleware::{DynMiddleware, Middleware};
use crate::verbs::{Map, Verb};
use radr::Location;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Default)]
pub struct Route {
    pub(crate) verbs: Map<DynMiddleware>,
    pub(crate) stack: Vec<DynMiddleware>,
}

pub struct Router<'a> {
    pub(crate) root: Location<'a, Route>,
}

pub trait Service: Send + Sync + 'static {
    fn mount(self: Arc<Self>, to: &mut Router);
}

impl Route {
    pub fn expose(&mut self, verb: Verb, action: impl Middleware) {
        self.verbs.insert(verb, Arc::new(action));
    }

    pub fn include(&mut self, middleware: impl Middleware) {
        self.stack.push(Arc::new(middleware));
    }
}

impl<'a> Router<'a> {
    pub fn at(&mut self, pattern: &'static str) -> Router {
        Router {
            root: self.root.at(pattern),
        }
    }

    pub fn mount(&mut self, service: impl Service) -> &mut Self {
        Service::mount(Arc::new(service), self);
        self
    }
}

impl<'a> Deref for Router<'a> {
    type Target = Route;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl<'a> DerefMut for Router<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root
    }
}
