use std::path::Path;

use super::Middleware;
use crate::response::File;

/// # Example
///
/// ```
/// use via::middleware::favicon;
/// use via::{Next, Request};
///
/// type Error = Box<dyn std::error::Error + Send + Sync>;
///
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() -> Result<(), Error> {
///     let mut app = via::app(());
///
///     app.at("/favicon.ico").respond(favicon("./favicon.ico"));
///     Ok(())
/// }
/// ```
pub fn favicon<T>(path: impl AsRef<Path>) -> impl Middleware<T> {
    let path_to_favicon = path.as_ref().to_path_buf();
    let content_type = match path_to_favicon
        .extension()
        .and_then(|os_str| os_str.to_str())
    {
        Some("ico") => "image/x-icon".to_owned(),
        Some("png") => "image/png".to_owned(),
        Some("svg") => "image/svg+xml".to_owned(),
        unsupported => {
            panic!("unsupported favicon format {:?}.", unsupported);
        }
    };

    crate::get(move |_, _| {
        File::open(&path_to_favicon)
            .mime_type(&content_type)
            .serve()
    })
}
