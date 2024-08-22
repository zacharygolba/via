use via::{Next, Request, Result};

async fn hello(mut request: Request, _: Next) -> Result<String> {
    // Attempt to parse the query parameter `n` to a `usize`. If the query
    // parameter doesn't exist or can't be converted to a `usize`, default to 1.
    let n = request.query("n").first().parse().unwrap_or(1);
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").required()?;
    // Create a greeting message that includes the name from the request's uri.
    let message = format!("Hello, {}!\n", name);

    // Send a plain text response with our greeting message, repeated `n` times.
    Ok(message.repeat(n))
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

    app.listen(("127.0.0.1", 8080), |address| {
        println!("Server listening at http://{}", address);
    })
    .await
}
