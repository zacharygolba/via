use via::http::header::CONTENT_TYPE;
use via::{Next, Request, Response, Result};

async fn echo(request: Request, _: Next) -> Result<Response> {
    // Optionally get the value of the Content-Type header from `request`.
    let content_type = request.headers().get(CONTENT_TYPE).cloned();
    // Get a stream of bytes from the body of the request.
    let body_stream = request.into_body().into_stream();

    // Stream the request body back to the client.
    Response::build()
        .stream(body_stream)
        .headers(Some(CONTENT_TYPE).zip(content_type))
        .finish()
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app(());

    // Add our echo responder to the endpoint /echo.
    app.at("/echo").respond(via::post(echo));
    //                           ^^^^
    // You can specify the HTTP method that middleware should accept with the
    // helper functions at the top-level of the `via` crate. In this case, the
    // `via::post` function is used to specify that the `echo` middleware should
    // only accept POST requests.

    app.listen(("127.0.0.1", 8080), |address| {
        println!("Server listening at http://{}", address);
    })
    .await
}
