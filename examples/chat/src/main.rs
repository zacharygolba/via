mod chat;
mod room;

use http::header;
use std::process::ExitCode;
use via::{App, Error, Response, Server};

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

    // Define a router namespace for our chat API.
    app.route("/chat/:room").scope(|route| {
        // GET / -> list all the messages in the room.
        route.respond(via::get(room::index));

        // GET /join -> websocket to read / write messages.
        route.route("/join").respond(via::ws(room::join));

        // Get /:index -> show the message with the provided index.
        route.route("/:index").respond(via::get(room::show));
    });

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
