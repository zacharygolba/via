use std::process::ExitCode;
use via::ws::{self, Channel, Message};
use via::{Error, Next, Request, Response, Server};

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    let mut app = via::app(());

    // Define a route that listens on /hello/:name.
    app.route("/hello/:name").to(via::get(hello));
    app.route("/chat").to(ws::upgrade(chat));

    Server::new(app).listen(("127.0.0.1", 8080)).await
}

async fn chat(mut channel: Channel, _: ws::Request) -> ws::Result {
    while let Some(message) = channel.recv().await {
        let Message::Text(text) = message else {
            continue;
        };

        println!("{}", text);
    }

    Ok(())
}

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}
