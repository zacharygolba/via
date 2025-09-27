use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use http::StatusCode;

use crate::error::{Error, Errors};
use crate::middleware::{BoxFuture, Middleware};
use crate::response::{Response, ResponseBuilder};
use crate::{Next, Pipe, Request};

/// Recover from errors that occur in downstream middleware.
///
pub struct Rescue<F> {
    recover: F,
}

/// Customize how an [`Error`] is converted to a response.
///
pub struct Sanitize<'a> {
    json: bool,
    error: &'a Error,
    status: Option<StatusCode>,
    message: Option<Cow<'a, str>>,
}

/// Recover from errors that occur in downstream middleware.
///
pub fn rescue<F>(recover: F) -> Rescue<F>
where
    F: Fn(Sanitize) -> Sanitize + Copy + Send + Sync + 'static,
{
    Rescue { recover }
}

impl<State, F> Middleware<State> for Rescue<F>
where
    State: Send + Sync + 'static,
    F: Fn(Sanitize) -> Sanitize + Copy + Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let Self { recover } = *self;

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                let response = Response::build();
                let sanitize = Sanitize::new(&error);

                recover(sanitize).pipe(response).or_else(|residual| {
                    if cfg!(debug_assertions) {
                        eprintln!("warn: a residual error occurred in rescue");
                        eprintln!("{}", residual);
                    }

                    Ok(error.into())
                })
            })
        })
    }
}

impl<'a> Sanitize<'a> {
    /// Generate a json response for the error.
    ///
    pub fn as_json(self) -> Self {
        Self { json: true, ..self }
    }

    /// Sanitize the contained error based on the error source.
    ///
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(Self, &(dyn std::error::Error + 'static)) -> Self,
    {
        if let Some(source) = self.error.source() {
            f(self, source)
        } else {
            self
        }
    }

    /// Use the canonical reason of the status code as the error message.
    ///
    pub fn with_canonical_reason(self) -> Self {
        Self {
            message: self.status_code().canonical_reason().map(Cow::Borrowed),
            ..self
        }
    }

    /// Provide a custom message to use for the response generated from this
    /// error.
    ///
    pub fn with_message<T>(self, message: T) -> Self
    where
        Cow<'a, str>: From<T>,
    {
        Self {
            message: Some(message.into()),
            ..self
        }
    }

    /// Overrides the HTTP status code of the error.
    ///
    pub fn with_status_code(self, status: StatusCode) -> Self {
        Self {
            status: Some(status),
            ..self
        }
    }
}

impl<'a> Sanitize<'a> {
    fn new(error: &'a Error) -> Self {
        Self {
            json: false,
            error,
            status: None,
            message: None,
        }
    }

    fn status_code(&self) -> StatusCode {
        self.status.unwrap_or(self.error.status)
    }
}

impl Display for Sanitize<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self.error, f)
    }
}

impl Pipe for Sanitize<'_> {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        let status_code = self.status_code();
        let response = response.status(status_code);

        match self.message {
            None if self.json => response.json(&self.error.repr_json(status_code)),
            Some(message) if self.json => response.json(Errors::new(status_code).push(message)),

            None => response.text(self.error.to_string()),
            Some(message) => response.text(message),
        }
    }
}
