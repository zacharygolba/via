use std::sync::Arc;

use crate::middleware::Middleware;

#[cfg(feature = "ws")]
use crate::error::BoxError;
#[cfg(feature = "ws")]
use crate::ws::{WebSocket, WsConfig};

pub type Router<T> = via_router::Router<Arc<dyn Middleware<T>>>;

pub struct Route<'a, T> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<T>>>,
}

impl<T> Route<'_, T> {
    pub fn at(&mut self, path: &'static str) -> Route<'_, T> {
        Route {
            inner: self.inner.at(path),
        }
    }

    pub fn scope(&mut self, scope: impl FnOnce(&mut Self)) {
        scope(self);
    }

    pub fn include<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.include(Arc::new(middleware));
    }

    pub fn respond<M>(&mut self, middleware: M)
    where
        M: Middleware<T> + 'static,
    {
        self.inner.respond(Arc::new(middleware));
    }
}

#[cfg(feature = "ws")]
impl<T: Send + Sync + 'static> Route<'_, T> {
    pub fn ws<R, F>(&mut self, on_message: F)
    where
        F: Fn(WebSocket<T>, Option<String>) -> R + Send + Sync + 'static,
        R: Future<Output = Result<(), BoxError>> + Send + Sync + 'static,
    {
        self.respond(WsConfig::new(
            self.inner.param().map(|param| param.to_owned()),
            Arc::new(move |socket, message| Box::pin(on_message(socket, message))),
        ));
    }
}
