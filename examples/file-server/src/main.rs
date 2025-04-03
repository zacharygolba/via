use std::path::PathBuf;
use std::process::ExitCode;
use via::middleware::error_boundary;
use via::response::File;
use via::{Next, Request};

/// The maximum amount of memory that will be allocated to serve a single file.
///
/// For the purpose of demonstrating the `File::serve` fn, files larger than
/// `1 MiB` will be streamed. In a production app, I would probably set this
/// to `10 MiB`.
///
const MAX_ALLOC_SIZE: usize = 1024 * 1024;

/// The relative path of the public directory in relationship to the current
/// working directory of the process.
///
const PUBLIC_DIR: &str = "./public";

type Error = Box<dyn std::error::Error + Send + Sync>;

/// Serve the file at the provided path argument.
///
async fn file_server(request: Request, _: Next) -> via::Result {
    let path_param = request.param("path").percent_decode().into_result()?;
    let file_path = resolve_path(path_param.as_ref());
    let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();

    File::open(&file_path)
        .content_type(mime_type.to_string())
        .with_last_modified()
        .serve(MAX_ALLOC_SIZE)
        .await
}

/// Simple yet familiar file path resolution...
///
fn resolve_path(path_param: &str) -> PathBuf {
    let mut path = PathBuf::from(PUBLIC_DIR).join(if path_param.is_empty() {
        // Assume an empty path param means a request to index.html.
        "index.html"
    } else {
        path_param
    });

    // If the path doesn't have an extension, assume it's an HTML file.
    if path.extension().is_none() {
        path.set_extension("html");
    }

    path
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Create a new application.
    let mut app = via::app(());

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::map(|error| {
        eprintln!("error: {}", error);
        error.use_canonical_reason()
    }));

    // Serve any file located in the public dir.
    app.at("/*path").respond(via::get(file_server));

    Ok(via::start(app).listen(("127.0.0.1", 8080)).await?)
}
