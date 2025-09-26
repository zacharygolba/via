use std::process::ExitCode;
use via::{App, BoxError, Next, Pipe, Request, Response, rescue};

async fn echo(request: Request, _: Next) -> via::Result {
    request.pipe(Response::build())
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    let mut app = App::new(());

    // Capture errors from downstream, log them, and map them into responses.
    // Upstream middleware remains unaffected and continues execution.
    app.include(rescue::inspect(|error| eprintln!("error: {}", error)));

    // Add our echo responder to the endpoint /echo.
    app.at("/echo").respond(via::post(echo));
    //                           ^^^^
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate. In this case, the
    // `via::post` function is used to specify that the `echo` middleware should
    // only accept POST requests.

    via::serve(app).listen(("127.0.0.1", 8080)).await
}
