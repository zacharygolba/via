use via::prelude::*;

#[via::route("/*path")]
async fn echo(path: String, context: Context) -> String {
    format!("{} {}", context.method(), path)
}

fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.at("/").scope(echo);
    app.listen()
}
