use via::prelude::*;

#[via::get("/")]
async fn hello() -> &'static str {
    "Hello, world!"
}

fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.at("/").scope(hello);
    app.listen()
}
