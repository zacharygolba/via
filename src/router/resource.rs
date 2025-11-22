use http::Method;

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request};

pub struct Collection<Index, Create> {
    exhaustive: bool,
    index: Index,
    create: Create,
}

pub struct Member<Show, Update, Destroy> {
    exhaustive: bool,
    show: Show,
    update: Update,
    destroy: Destroy,
}

#[derive(Debug)]
pub(crate) struct MethodNotAllowed {
    allow: &'static str,
    method: Method,
}

#[macro_export]
macro_rules! resources {
    ($module:path) => {
        (
            $crate::resources!($module as collection),
            $crate::resources!($module as member),
        )
    };
    ($module:path as collection) => {{
        use $module::{create, index};
        $crate::get(index).post(create)
    }};
    ($module:path as member) => {{
        use $module::{destroy, show, update};
        $crate::get(show).patch(update).delete(destroy)
    }};
    ($module:path as $other:ident) => {{
        compile_error!(concat!(
            "incorrect rest! modifier \"",
            stringify!($other),
            "\"",
        ));
    }};
}

impl<Index, Create> Collection<Index, Create> {
    #[doc(hidden)]
    pub fn new(index: Index, create: Create) -> Self {
        Self {
            exhaustive: true,
            index,
            create,
        }
    }

    pub fn or_next(self) -> Self {
        Self {
            exhaustive: false,
            ..self
        }
    }

    fn method_not_allowed(&self, method: &Method) -> Box<MethodNotAllowed> {
        Box::new(MethodNotAllowed {
            allow: "GET, POST",
            method: method.clone(),
        })
    }
}

impl<App, Index, Create> Middleware<App> for Collection<Index, Create>
where
    Index: Middleware<App>,
    Create: Middleware<App>,
{
    fn call(&self, request: Request<App>, next: Next<App>) -> BoxFuture {
        match *request.envelope().method() {
            Method::GET => self.index.call(request, next),
            Method::POST => self.create.call(request, next),

            ref method => {
                if self.exhaustive {
                    let error = self.method_not_allowed(method).into();
                    Box::pin(async { Err(error) })
                } else {
                    next.call(request)
                }
            }
        }
    }
}

impl<Show, Update, Destroy> Member<Show, Update, Destroy> {
    #[doc(hidden)]
    pub fn new(show: Show, update: Update, destroy: Destroy) -> Self {
        Self {
            exhaustive: true,
            show,
            update,
            destroy,
        }
    }

    pub fn or_next(self) -> Self {
        Self {
            exhaustive: false,
            ..self
        }
    }

    fn method_not_allowed(&self, method: &Method) -> Box<MethodNotAllowed> {
        Box::new(MethodNotAllowed {
            allow: "DELETE, GET, PATCH",
            method: method.clone(),
        })
    }
}

impl<App, Show, Update, Destroy> Middleware<App> for Member<Show, Update, Destroy>
where
    Show: Middleware<App>,
    Update: Middleware<App>,
    Destroy: Middleware<App>,
{
    fn call(&self, request: Request<App>, next: Next<App>) -> BoxFuture {
        match *request.envelope().method() {
            Method::GET => self.show.call(request, next),
            Method::PATCH => self.update.call(request, next),
            Method::DELETE => self.destroy.call(request, next),

            ref method => {
                if self.exhaustive {
                    let error = self.method_not_allowed(method).into();
                    Box::pin(async { Err(error) })
                } else {
                    next.call(request)
                }
            }
        }
    }
}

impl MethodNotAllowed {
    pub fn allow(&self) -> &str {
        self.allow
    }

    pub fn method(&self) -> &Method {
        &self.method
    }
}
