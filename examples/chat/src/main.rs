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

    app.route("/").to(via::get(async |_, _| {
        Response::build()
            .header(header::CONTENT_SECURITY_POLICY, CSP)
            .text("Chat Example frontend coming soon!")
    }));

    // Define the router namespace for our chat API.
    {
        let mut room = app.route("/chat/:room");

        // list all the messages in the room.
        room.route("/").to(via::get(room::index));

        // websocket to read / write messages.
        room.route("/join").to(via::ws(room::join));

        // show the message with the provided index.
        room.route("/:index").to(via::get(room::show));
    }

    Server::new(app).listen(("127.0.0.1", 8080)).await
}
