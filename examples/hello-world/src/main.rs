use via::prelude::*;

#[get("/")]
async fn hello() -> &'static str {
    "Hello, world!"
}

fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.mount(hello);
    app.listen()
}
