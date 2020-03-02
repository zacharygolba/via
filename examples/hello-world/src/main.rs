use via::prelude::*;

#[via::expose(GET, "/")]
async fn hello() -> impl Respond {
    "Hello, world!"
}

fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.mount(hello);
    app.listen()
}
