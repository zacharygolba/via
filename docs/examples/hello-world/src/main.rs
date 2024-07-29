use via::{Event, Next, Request, Response, Result};

async fn hello(request: Request, _: Next) -> Result<Response> {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").required()?;
    //                               ^^^^^^^^
    // Calling `required` here converts the PathParam wrapper into a result.
    // if there is no path parameter named `name` in the request uri, an
    // error will be returned to the client with a 400 status code.
    //
    // In this example application, the `name` path parameter will always be
    // present since this middleware function is only added to the endpoint
    // /hello/:name.

    // Send a plain text response with a greeting that includes the name from
    // the request's uri path.
    Response::text(format!("Hello, {}!", name)).finish()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new app by calling the `via::app` function.
    let mut app = via::app(());
    //                     ^^
    // Shared state can be passed to the app by passing a value to the
    // `via::app` function. Check out the shared-state example for more
    // information.

    // Add our hello responder to the endpoint /hello/:name. Middleware that is
    // added to an endpoint with `.respond()` will only run if a request's path
    // matches the path of the endpoint exactly.
    app.at("/hello/:name").respond(via::get(hello));
    //             ^^^^^           ^^^^^^^^
    // When defining an endpoint with a colon prefix in a path segment, the
    // path segment is treated as dynamic and will match any value up to the
    // next path segment or the end of the request uri. A reference to the path
    // path parameter will be available in the requests params under the name
    // that immediately follows the colon in the path segment (i.e "name").
    //
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate.

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
