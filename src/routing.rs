use crate::{handler::ArcMiddleware, Context, Middleware, Next, Result};
use futures::future::BoxFuture;
use http::Extensions;
use std::sync::Arc;
use verbs::*;

pub trait Service: Send + Sync + 'static {
    fn mount(self: Arc<Self>, location: &mut Location);
}

pub struct Location<'a> {
    state: &'a mut Extensions,
    value: radr::Location<'a, Endpoint>,
}

#[derive(Default)]
pub(crate) struct Endpoint {
    verbs: Map<ArcMiddleware>,
    stack: Vec<ArcMiddleware>,
}

#[derive(Default)]
pub(crate) struct Router {
    routes: radr::Router<Endpoint>,
}

impl Endpoint {
    pub fn expose(&mut self, verb: Verb, handler: impl Middleware) -> &mut Self {
        self.verbs.insert(verb, Arc::new(handler));
        self
    }

    pub fn middleware(&mut self, handler: impl Middleware) -> &mut Self {
        self.stack.push(Arc::new(handler));
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

impl Router {
    #[inline]
    pub fn at<'a>(&'a mut self, state: &'a mut Extensions, path: &'static str) -> Location<'a> {
        Location {
            state,
            value: self.routes.at(path),
        }
    }

    #[inline]
    pub fn visit(&self, mut context: Context) -> BoxFuture<'static, Result> {
        let (parameters, method, path) = context.locate();
        let matches = self.routes.visit(path).flat_map(|matched| {
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
}
