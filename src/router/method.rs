use bitflags::bitflags;
use std::fmt::{self, Display, Formatter};

use crate::middleware::{BoxFuture, Middleware};
use crate::{Error, Next, Request};

/// Call the next middleware in the stack.
///
pub struct Continue;

pub struct And<T, U> {
    middleware: T,
    or_else: U,
    mask: Mask,
}

pub struct Method<T> {
    middleware: T,
    mask: Mask,
}

pub struct NotAllowed {
    allow: Mask,
}

#[derive(Debug)]
pub(crate) struct MethodNotAllowed {
    allow: Mask,
    method: Mask,
}

macro_rules! methods {
    ($($vis:vis fn $name:ident($method:ident));+ $(;)?) => {
        $(
            #[doc = concat!(
                "Route `",
                stringify!($method),
                "` requests to the provided middleware."
            )]
            $vis fn $name<T>(middleware: T) -> And<Method<T>, Continue> {
                let mask = Mask::$method;

                And {
                    middleware: Method { middleware, mask },
                    or_else: Continue,
                    mask,
                }
            }
        )+
    };
    ($($vis:vis fn $name:ident($self:ident, $method:ident));+ $(;)?) => {
        $($vis fn $name<F>($self, middleware: F) -> And<Method<F>, Self> {
            $self.and(Mask::$method, middleware)
        })+
    };
}

trait Predicate {
    fn matches(&self, other: Mask) -> bool;
}

bitflags! {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Mask: u16 {
        const CONNECT = 1 << 0;
        const DELETE  = 1 << 1;
        const GET     = 1 << 2;
        const HEAD    = 1 << 3;
        const OPTIONS = 1 << 4;
        const PATCH   = 1 << 5;
        const POST    = 1 << 6;
        const PUT     = 1 << 7;
        const TRACE   = 1 << 8;
    }
}

methods! {
    pub fn connect(CONNECT);
    pub fn delete(DELETE);
    pub fn get(GET);
    pub fn head(HEAD);
    pub fn options(OPTIONS);
    pub fn patch(PATCH);
    pub fn post(POST);
    pub fn put(PUT);
    pub fn trace(TRACE);
}

impl<T, U> And<T, U> {
    methods! {
        pub fn connect(self, CONNECT);
        pub fn delete(self, DELETE);
        pub fn get(self, GET);
        pub fn head(self, HEAD);
        pub fn options(self, OPTIONS);
        pub fn patch(self, PATCH);
        pub fn post(self, POST);
        pub fn put(self, PUT);
        pub fn trace(self, TRACE);
    }

    /// Returns a `405 Method Not Allowed` response if the request method is
    /// not supported.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Next, Request, Response};
    /// #
    /// # async fn greet(request: Request, _: Next) -> via::Result {
    /// #   let name = request.envelope().param("name").into_result()?;
    /// #   Response::build().text(format!("Hello, {}!", name))
    /// # }
    /// #
    /// # fn main() {
    /// # let mut app = App::new(());
    /// app.route("/hello/:name").to(via::get(greet).or_not_allowed());
    /// // curl -XPOST http://localhost:8080/hello/world
    /// // => Method Not Allowed: POST
    /// # }
    /// ```
    ///
    pub fn or_not_allowed(self) -> And<Self, NotAllowed> {
        let allow = self.mask;

        And {
            middleware: self,
            or_else: NotAllowed { allow },
            mask: allow,
        }
    }

    fn and<F>(self, mask: Mask, middleware: F) -> And<Method<F>, Self> {
        And {
            mask: self.mask | mask,
            or_else: self,
            middleware: Method { middleware, mask },
        }
    }
}

impl Mask {
    fn as_str(&self) -> Option<&str> {
        match *self {
            Mask::CONNECT => Some("CONNECT"),
            Mask::DELETE => Some("DELETE"),
            Mask::GET => Some("GET"),
            Mask::HEAD => Some("HEAD"),
            Mask::OPTIONS => Some("OPTIONS"),
            Mask::PATCH => Some("PATCH"),
            Mask::POST => Some("POST"),
            Mask::PUT => Some("PUT"),
            Mask::TRACE => Some("TRACE"),
            _ => None,
        }
    }
}

impl MethodNotAllowed {
    pub fn allows(&self) -> String {
        self.allow.iter().fold(String::new(), |allow, mask| {
            let Some(method) = mask.as_str() else {
                return allow;
            };

            if allow.is_empty() {
                allow + method
            } else {
                allow + ", " + method
            }
        })
    }
}

impl<T, U> Predicate for And<T, U> {
    #[inline]
    fn matches(&self, other: Mask) -> bool {
        self.mask.contains(other)
    }
}

impl<T> Predicate for Method<T> {
    #[inline]
    fn matches(&self, other: Mask) -> bool {
        self.mask.contains(other)
    }
}

impl<State, T, U> Middleware<State> for And<T, U>
where
    T: Middleware<State> + Predicate,
    U: Middleware<State>,
{
    #[inline(always)]
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let method = request.envelope().method();
        let mask = Mask::from(method);

        if self.middleware.matches(mask) {
            self.middleware.call(request, next)
        } else {
            self.or_else.call(request, next)
        }
    }
}

impl<State> Middleware<State> for Continue {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        next.call(request)
    }
}

impl<State, T> Middleware<State> for Method<T>
where
    T: Middleware<State>,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        self.middleware.call(request, next)
    }
}

impl<State> Middleware<State> for NotAllowed {
    fn call(&self, request: Request<State>, _: Next<State>) -> BoxFuture {
        let error = Error::method_not_allowed(MethodNotAllowed {
            allow: self.allow,
            method: request.envelope().method().into(),
        });

        Box::pin(async { Err(error) })
    }
}

impl From<&'_ http::Method> for Mask {
    fn from(method: &http::Method) -> Self {
        match *method {
            http::Method::CONNECT => Mask::CONNECT,
            http::Method::DELETE => Mask::DELETE,
            http::Method::GET => Mask::GET,
            http::Method::HEAD => Mask::HEAD,
            http::Method::OPTIONS => Mask::OPTIONS,
            http::Method::PATCH => Mask::PATCH,
            http::Method::POST => Mask::POST,
            http::Method::PUT => Mask::PUT,
            http::Method::TRACE => Mask::TRACE,
            _ => Mask::empty(),
        }
    }
}

impl std::error::Error for MethodNotAllowed {}

impl Display for MethodNotAllowed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(method) = self.method.as_str() {
            write!(f, "method not allowed: \"{}\"", method)
        } else {
            write!(f, "method not allowed")
        }
    }
}
