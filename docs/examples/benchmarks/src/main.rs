use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app();

    app.at("/text").respond(via::get(|_, _| async {
        Response::text("Hello, world!").end()
    }));

    let mut unit = app.at("/unit");
    unit.respond(via::get(|_, _| async { Response::new().status(204).end() }));

    app.listen(("127.0.0.1", 8080)).await
}
