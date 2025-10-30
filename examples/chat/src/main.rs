mod chat;
mod room;

use http::header;
use std::process::ExitCode;
use via::{App, Error, Response, Server, error::Rescue};

use crate::chat::Chat;

const CSP: &str = "default-src 'self'; connect-src 'self'";

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = App::new(Chat::new());

    app.route("/").respond(via::get(async |_, _| {
        Response::build()
            .header(header::CONTENT_SECURITY_POLICY, CSP)
            .text("Chat Example frontend coming soon!")
    }));

    let mut chat = app.route("/chat");

    chat.middleware(Rescue::with(|sanitizer| sanitizer.use_json()));

    // Define a router namespace for our chat API.
    chat.route("/:room").scope(|route| {
        // GET / -> list all the messages in the room.
        route.respond(via::get(room::index));

        // GET /join -> websocket to read / write messages.
        route.route("/join").respond(via::websocket(room::join));

        // Get /:index -> show the message with the provided index.
        route.route("/:index").respond(via::get(room::show));
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
