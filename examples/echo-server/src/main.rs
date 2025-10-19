use std::process::ExitCode;
use via::{App, Error, Next, Pipe, Request, Response, Server};

async fn echo(request: Request, _: Next) -> via::Result {
    request.pipe(Response::build())
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = App::new(());

    // Add our echo responder to the endpoint /echo.
    app.route("/echo").respond(via::post(echo));
    //                           ^^^^
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate. In this case, the
    // `via::post` function is used to specify that the `echo` middleware should
    // only accept POST requests.

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
