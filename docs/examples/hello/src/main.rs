use via::prelude::*;

struct Routes;

#[service]
impl Routes {
    includes! {
        // middleware::cookies(b"ri30r90923r2r90r09eqJC0[09EF9EFJA9EFJA9WEJFEWF"),
    }

    #[action(GET, "/hello/:name")]
    async fn hello(name: String, context: Context) -> Result<impl Respond> {
        println!("{:?}", context);
        // context.cookies()?.add("test=true".parse()?)?;
        Ok(format!("Hello, {}", name))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(Routes);
    app.listen(("0.0.0.0", 8080)).await
}
