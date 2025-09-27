mod chat;
mod room;

use http::header;
use std::process::ExitCode;
use via::{App, BoxError, Response};

use crate::chat::Chat;

const CSP: &str = "default-src 'self'; connect-src 'self'";

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    let mut app = App::new(Chat::new());

    app.at("/").respond(via::get(async |_, _| {
        Response::build()
            .header(header::CONTENT_SECURITY_POLICY, CSP)
            .text("Chat Example frontend coming soon!")
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

    via::serve(app).listen(("127.0.0.1", 8080)).await
}
