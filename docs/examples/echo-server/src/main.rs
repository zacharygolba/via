use via::{http::header, Event, Next, Request, Response, Result};

async fn echo(mut request: Request, _: Next) -> Result<Response> {
    // Read the request body into a Vec<u8>.
    let request_body = request.body_mut().read_bytes().await?;
    // Get a reference to the request headers.
    let request_headers = request.headers();
    // Create a new vector to store the response headers.
    let mut response_headers = Vec::new();

    if let Some(content_type) = request_headers.get(header::CONTENT_TYPE) {
        // Add the Content-Type header from the request to the response.
        response_headers.push((header::CONTENT_TYPE, content_type.clone()));
    }

    if let Some(content_length) = request_headers.get(header::CONTENT_LENGTH) {
        // Add the Content-Length header from the request to the response.
        response_headers.push((header::CONTENT_LENGTH, content_length.clone()));
    }

    // Send the request body back to the client in the response.
    Response::builder()
        .headers(response_headers)
        .body(request_body)
        .end()
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
