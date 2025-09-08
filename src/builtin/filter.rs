use crate::{BoxFuture, Middleware, Next, Request};

pub trait Predicate<T>: Send + Sync {
    fn matches(&self, request: &Request<T>) -> bool;
}

pub struct Filter<P, M> {
    predicate: P,
    middleware: M,
}

pub fn filter<M, T, P>(predicate: P, middleware: M) -> Filter<P, M>
where
    M: Middleware<T>,
    P: Predicate<T>,
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

impl<M, T, P> Middleware<T> for Filter<P, M>
where
    M: Middleware<T>,
    P: Predicate<T>,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        if self.predicate.matches(&request) {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
