use crate::{handler::DynHandler, Context, Future, Handler, Next};
use http::Extensions;
use verbs::{Map, Verb};

pub(crate) type Router = radr::Router<Endpoint>;

pub trait Service: Send + Sync + 'static {
    fn mount(&self, location: &mut Location);
}

pub struct Location<'a> {
    pub(crate) state: &'a mut Extensions,
    pub(crate) value: radr::Location<'a, Endpoint>,
}

#[derive(Default)]
pub(crate) struct Endpoint {
    verbs: Map<DynHandler>,
    stack: Vec<DynHandler>,
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
    pub fn expose(&mut self, verb: Verb, handler: impl Handler) -> &mut Self {
        self.verbs.insert(verb, Box::new(handler));
        self
    }

    pub fn middleware(&mut self, handler: impl Handler) -> &mut Self {
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
    pub fn expose(&mut self, verb: Verb, handler: impl Handler) {
        self.value.expose(verb, handler);
    }

    #[inline]
    pub fn middleware(&mut self, handler: impl Handler) {
        self.value.middleware(handler);
    }

    #[inline]
    pub fn mount(&mut self, service: impl Service) {
        service.mount(self);
        self.state.insert(service);
    }
}
