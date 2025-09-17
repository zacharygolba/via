mod chat;
mod room;

use http::header;
use std::process::ExitCode;
use via::builtin::rescue;
use via::{BoxError, Response};

use crate::chat::Chat;

const CSP: &str = "default-src 'self'; connect-src 'self'";

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Create a new application.
    let mut app = via::app(Chat::new());

    // Capture errors from downstream, log them, and map them into responses.
    // Upstream middleware remains unaffected and continues execution.
    app.include(rescue::map(|error| {
        eprintln!("error: {}", error);
        error.as_json()
    }));

    app.at("/").respond(via::get(async |_, _| {
        Response::build()
            .header(header::CONTENT_SECURITY_POLICY, CSP)
            .text("Chat Example frontend coming soon!".to_owned())
    }));

    // Define a router namespace for our chat API.
    app.at("/chat/:room").scope(|route| {
        // GET / -> list all the messages in the room.
        route.respond(via::get(room::index));

        // GET /join -> websocket to read / write messages.
        route.at("/join").respond(via::ws(room::join));

        // Get /:index -> show the message with the provided index.
        route.at("/:index").respond(via::get(room::show));
    });

    via::start(app).listen(("127.0.0.1", 8080)).await
}
