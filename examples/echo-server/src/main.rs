use http::header::CONTENT_TYPE;
use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{Next, Pipe, Request, Response};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn echo(request: Request, _: Next) -> via::Result {
    let mut response = Response::build();

    // If a Content-Type header is present, include it in the response.
    if let Some(content_type) = request.header(CONTENT_TYPE).cloned() {
        response = response.header(CONTENT_TYPE, content_type);
    }

    // Stream the request payload back to the client with the options configured
    // in the response builder above.
    request.into_body().pipe(response)
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = via::app(());

    app.include(|request: Request, next: Next| {
        let request = request.tee(tokio::io::stderr());
        async { Ok(next.call(request).await?.tee(tokio::io::stderr())) }
    });

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::inspect(|_, error| {
        eprintln!("Error: {}", error);
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
