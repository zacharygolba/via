use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.at("/text").get(|_, _| async { "Hello, world!" });
    app.at("/unit").get(|_, _| async {});
    app.listen(("0.0.0.0", 8080)).await
}
