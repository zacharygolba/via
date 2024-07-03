use via::{Event, Next, Request, Response, Result};

async fn echo(request: Request, _: Next) -> Result<Response> {
    let method = request.method();
    let path = request.param("path").required()?;

    Response::text(format!("{} \"{}\"", method, path)).end()
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app(());

    // Add our echo responder to the endpoint /echo/*path.
    app.at("/echo/*path").respond(echo);
    //            ^^^^^
    // When defining an endpoint with a wildcard prefix in a path segment, any
    // request that matches the path prefix will be directed to the added through
    // either `.include()` or `.respond()`. A reference to the remaining request
    // uri path will be avaiable in the requests params under the name that
    // immediately follows the * character in the path segment.

    app.listen(("127.0.0.1", 8080), |event| match event {
        Event::ConnectionError(error) | Event::UncaughtError(error) => {
            eprintln!("Error: {}", error);
        }
        Event::ServerReady(address) => {
            println!("Server listening at http://{}", address);
        }
    })
    .await
}
