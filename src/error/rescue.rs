use http::StatusCode;
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use crate::error::{Error, Errors};
use crate::middleware::{BoxFuture, Middleware};
use crate::response::{Json, Response, ResponseBuilder};
use crate::{Next, Pipe, Request};

/// Recover from errors that occur in downstream middleware.
///
pub struct Rescue<F> {
    recover: F,
}

/// Customize how an [`Error`] is converted to a response.
///
pub struct Sanitizer<'a> {
    json: bool,
    error: &'a Error,
    status: Option<StatusCode>,
    message: Option<Cow<'a, str>>,
}

/// Recover from errors that occur in downstream middleware.
///
pub fn rescue<F>(recover: F) -> Rescue<F>
where
    F: Fn(&mut Sanitizer) + Copy + Send + Sync + 'static,
{
    Rescue { recover }
}

impl<State, F> Middleware<State> for Rescue<F>
where
    State: Send + Sync + 'static,
    F: Fn(&mut Sanitizer) + Copy + Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let Self { recover } = *self;

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                let response = Response::build();
                let mut sanitizer = Sanitizer::new(&error);

                recover(&mut sanitizer);
                sanitizer.pipe(response).or_else(|residual| {
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

impl<'a> Sanitizer<'a> {
    /// Returns a reference to the error source.
    ///
    pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }

    /// Generate a json response for the error.
    ///
    pub fn respond_with_json(&mut self) -> &mut Self {
        self.json = true;
        self
    }

    /// Provide a custom message to use for the response generated from this
    /// error.
    ///
    pub fn set_message<T>(&mut self, message: T) -> &mut Self
    where
        Cow<'a, str>: From<T>,
    {
        self.message = Some(message.into());
        self
    }

    /// Overrides the HTTP status code of the error.
    ///
    pub fn set_status_code(&mut self, status: StatusCode) -> &mut Self {
        self.status = Some(status);
        self
    }

    /// Use the canonical reason of the status code as the error message.
    ///
    pub fn use_canonical_reason(&mut self) -> &mut Self {
        self.message = self.status_code().canonical_reason().map(Cow::Borrowed);
        self
    }
}

impl<'a> Sanitizer<'a> {
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

impl Display for Sanitizer<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self.error, f)
    }
}

impl Pipe for Sanitizer<'_> {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        let status_code = self.status_code();
        let response = response.status(status_code);

        if self.json {
            let payload = self.message.map_or_else(
                || self.error.repr_json(status_code),
                |message| {
                    let mut errors = Errors::new(status_code);
                    errors.push(message);
                    errors
                },
            );

            Json(&payload).pipe(response)
        } else if let Some(message) = self.message {
            response.text(message)
        } else {
            response.text(self.error.to_string())
        }
    }
}
