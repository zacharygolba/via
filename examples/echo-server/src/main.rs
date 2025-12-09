use std::process::ExitCode;
use via::response::{Finalize, Response};
use via::{Error, Next, Request, Server};

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = via::app(());

    // Add our echo middleware to the route /echo.
    app.route("/echo").to(via::post(echo));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}

async fn echo(request: Request, _: Next) -> via::Result {
    request.finalize(Response::build())
}
