use super::{FutureResponse, Middleware, Next};
use crate::request::Request;

pub struct RespondTo<T> {
    predicate: http::Method,
    middleware: T,
}

pub fn connect<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::DELETE,
        middleware,
    }
}

pub fn delete<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::DELETE,
        middleware,
    }
}

pub fn get<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::GET,
        middleware,
    }
}

pub fn head<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::HEAD,
        middleware,
    }
}

pub fn options<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::OPTIONS,
        middleware,
    }
}

pub fn patch<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::PATCH,
        middleware,
    }
}

pub fn post<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::POST,
        middleware,
    }
}

pub fn put<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::PUT,
        middleware,
    }
}

pub fn trace<State, T>(middleware: T) -> RespondTo<T>
where
    T: Middleware<State>,
{
    RespondTo {
        predicate: http::Method::TRACE,
        middleware,
    }
}

impl<State, T> Middleware<State> for RespondTo<T>
where
    T: Middleware<State>,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> FutureResponse {
        if &self.predicate == request.method() {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
