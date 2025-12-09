use std::process::ExitCode;
use via::response::{Finalize, Response};
use via::{App, Error, Next, Request, Server};

async fn echo(request: Request, _: Next) -> via::Result {
    request.finalize(Response::build())
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = App::new(());

    // Add our echo middleware to the route /echo.
    app.route("/echo").respond(via::post(echo));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
