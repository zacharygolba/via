use via::error::BoxError;
use via::http::header::CONTENT_TYPE;
use via::{Next, Request, Response, Server};

async fn echo(request: Request, _: Next) -> via::Result<Response> {
    // Get an owned copy of the request's Content-Type header.
    let content_type = request.headers().get(CONTENT_TYPE).cloned();

    // Consume the request and get a stream of bytes from the body.
    let body_stream = request.into_body().stream();

    Response::build()
        .stream(body_stream)
        .headers(Some(CONTENT_TYPE).zip(content_type))
        .finish()
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let mut app = via::new(());

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
