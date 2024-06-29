use via::{Event, Response, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app();

    app.at("/text").respond(via::get(|_, _| async {
        Response::text("Hello, world!").end()
    }));

    app.at("/unit").respond(via::get(|_, _| async {
        Response::build().status(204).end()
    }));

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
