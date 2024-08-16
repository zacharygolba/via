use via::http::header;
use via::{Next, Request, Response, Result};

async fn echo(mut request: Request, _: Next) -> Result<Response> {
    // Get a stream of bytes from the request body.
    let body_stream = request.take_body()?.into_stream();
    // Optionally get the Content-Type header from the request.
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| (header::CONTENT_TYPE, value.clone()));

    // Stream the request body back to the client.
    Response::stream(body_stream).headers(content_type).finish()
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
