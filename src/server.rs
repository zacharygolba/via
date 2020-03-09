use crate::{runtime::MakeService, App, Result};
use hyper::Server;
use std::{env, net::SocketAddr};

struct Options {
    address: SocketAddr,
}

pub async fn serve(app: App) -> Result<()> {
    let options = Options::env()?;
    let server = Server::bind(&options.address).serve(MakeService::from(app));
    let ctrlc = async {
        let message = "failed to install CTRL+C signal handler";
        tokio::signal::ctrl_c().await.expect(message);
    };

    println!("Server listening at http://{}", options.address);
    Ok(server.with_graceful_shutdown(ctrlc).await?)
}

impl Options {
    fn env() -> Result<Options> {
        let mut host = "0.0.0.0".parse()?;
        let mut port = 8080;

        if let Some(value) = env::var("HOST").ok() {
            host = value.parse()?;
        }

        if let Some(value) = env::var("PORT").ok() {
            port = value.parse()?;
        }

        Ok(Options {
            address: SocketAddr::new(host, port),
        })
    }
}
