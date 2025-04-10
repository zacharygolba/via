use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{Next, Pipe, Request, Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn echo(request: Request, _: Next) -> via::Result {
    request.pipe(Response::build())
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = via::app(());

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::map(|error| {
        eprintln!("error: {}", error);
        error.use_canonical_reason()
    }));

    // Add our echo responder to the endpoint /echo.
    app.at("/echo").respond(via::post(echo));
    //                           ^^^^
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate. In this case, the
    // `via::post` function is used to specify that the `echo` middleware should
    // only accept POST requests.

    via::start(app).listen(("127.0.0.1", 8080)).await
}
