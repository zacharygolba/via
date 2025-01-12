use std::process::ExitCode;
use via::{BoxError, Next, Request, Response, Server};
use via_serve_static::serve_static;

async fn not_found(request: Request, _: Next) -> via::Result<Response> {
    let path = request.param("path").into_result()?;

    Response::build().status(404).html(format!(
        "
        <!DOCTYPE html>
        <html lang=\"en\">
            <head>
                <meta charset=\"UTF-8\" />
                <meta
                    name=\"viewport\"
                    content=\"width=device-width, initial-scale=1.0\"
                />
                <title>Not Found</title>
            </head>
            <body>
                <h1>File not found at path: \"{}\"</h1>
            </body>
        </html>
        ",
        path
    ))
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    let mut app = via::new(());

    // Add the serve_static middleware to the endpoint /*path.
    //
    // Typically you will want to define your static file serving middleware
    // after any other middleware or responders that match the same path. Doing
    // so would eliminate the need to access the filesystem for every request
    // that matches the path of the static file serving middleware. However,
    // for the purpose of demonstrating how fall through behavior works, we're
    // doing this a bit backwards in this example.
    serve_static(app.at("/*path"))
        // .chunked_read_timeout(60)
        // Uncomment the line above to set a custom timeout when streaming
        // files larger than the `eager_read_threshold`.
        //
        // .eager_read_threshold(1048576) // 1MB
        // Uncomment the line above to set a custom file size threshold at
        // which the serve_static middleware will switch from loading the
        // entire file into memory to streaming the file in chunks.
        //
        // .fall_through(false)
        // Uncomment the line above to prevent the responder added on line 62
        // from running if a file is not found.
        .serve("./public")?;
    //   ^^^^^
    // Serve the files in the ./public directory. The serve_static middleware
    // will attempt to find a file that matches the path parameter of the
    // request uri.

    // Optionally add a custom not found handler by leveraging the fall through
    // behavior of via and the via_serve_static middleware.
    app.at("/*path").respond(via::get(not_found));

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
