use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app();

    let mut text = app.at("/text");
    text.respond(via::get(|_, _| async { "Hello, world!" }));

    let mut unit = app.at("/unit");
    unit.respond(via::get(|_, _| async {}));

    app.listen(("127.0.0.1", 8080)).await
}
