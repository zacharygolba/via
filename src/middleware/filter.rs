use super::{BoxFuture, Middleware, Next};
use crate::request::Request;

pub trait Predicate<T>: Send + Sync {
    fn matches(&self, request: &Request<T>) -> bool;
}

pub struct Filter<P, M> {
    predicate: P,
    middleware: M,
}

pub fn filter<T, P, M>(predicate: P, middleware: M) -> Filter<P, M>
where
    P: Predicate<T>,
    M: Middleware<T>,
{
    Filter {
        predicate,
        middleware,
    }
}

impl<T, F> Predicate<T> for F
where
    F: Fn(&Request<T>) -> bool + Send + Sync,
{
    #[inline]
    fn matches(&self, request: &Request<T>) -> bool {
        (self)(request)
    }
}

impl<T, P, M> Middleware<T> for Filter<P, M>
where
    P: Predicate<T>,
    M: Middleware<T>,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        if self.predicate.matches(&request) {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
