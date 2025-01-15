use http::header::CONTENT_TYPE;
use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{Next, Pipe, Request, Response, Server};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn echo(request: Request, _: Next) -> via::Result {
    // Get an optional copy of the Content-Type header from the request.
    let content_type = request.header(CONTENT_TYPE).cloned();

    // Create a response builder with the Content-Type header from the request.
    let response = Response::build().headers([(CONTENT_TYPE, content_type)]);

    // Stream the request payload back to the client with the options configured
    // in the response builder above.
    request.into_body().pipe(response)
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = via::new(());

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::catch(|error, _| {
        eprintln!("Error: {}", error);
    }));

    // Add our echo responder to the endpoint /echo.
    app.at("/echo").respond(via::post(echo));
    //                           ^^^^
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate. In this case, the
    // `via::post` function is used to specify that the `echo` middleware should
    // only accept POST requests.

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
