use via::prelude::*;

#[expose(GET, "/")]
async fn hello() -> impl Respond {
    "Hello, world!"
}

fn main() -> Result<()> {
    let mut app = App::new();

    app.mount(hello);
    app.listen()
}
