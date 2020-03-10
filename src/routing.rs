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

impl<'a> Location<'a> {
    pub fn at(&mut self, path: &'static str) -> Location {
        Location {
            state: self.state,
            value: self.value.at(path),
        }
    }

    #[doc(hidden)]
    pub fn expose(&mut self, verb: Verb, middleware: impl Middleware) {
        self.value.verbs.insert(verb, Arc::new(middleware));
    }

    pub fn middleware(&mut self, middleware: impl Middleware) {
        self.value.stack.push(Arc::new(middleware));
    }

    pub fn service(&mut self, service: impl Service) {
        Service::mount(Arc::new(service), self);
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
