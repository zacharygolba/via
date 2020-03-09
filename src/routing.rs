use crate::{handler::DynMiddleware, http::Extensions, Context, Future, Middleware, Next};
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
    verbs: Map<DynMiddleware>,
    stack: Vec<DynMiddleware>,
}

pub(crate) fn visit(router: &Router, mut context: Context) -> Future {
    let (parameters, method, path) = context.locate();
    let matches = router.visit(path).flat_map(|matched| {
        let verbs = matched.verbs.get(if matched.exact {
            method.into()
        } else {
            Verb::none()
        });

        match matched.param {
            Some(("", _)) | Some((_, "")) | None => {}
            Some((name, value)) => {
                parameters.insert(name, value.to_owned());
            }
        }

        matched.stack.iter().chain(verbs)
    });

    Next::new(matches).call(context)
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
