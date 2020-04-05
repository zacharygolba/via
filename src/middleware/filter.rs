use crate::{BoxFuture, Context, Middleware, Next, Result};
use verbs::Verb;

pub struct Only<T: Middleware> {
    middleware: T,
    predicate: Verb,
}

pub fn only<T: Middleware>(predicate: Verb) -> impl FnOnce(T) -> Only<T> {
    move |middleware| Only {
        middleware,
        predicate,
    }
}

impl<T: Middleware> Middleware for Only<T> {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        let method = context.method().into();

        if self.predicate.intersects(method) {
            self.middleware.call(context, next)
        } else {
            next.call(context)
        }
    }
}
