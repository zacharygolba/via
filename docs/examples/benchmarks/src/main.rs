use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    let mut text = app.at("/text");
    text.respond(via::get(|_, _| async { "Hello, world!" }));

    let mut unit = app.at("/unit");
    unit.respond(via::get(|_, _| async {}));

    app.listen(("0.0.0.0", 8080)).await
}
