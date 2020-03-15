use crate::{runtime::MakeService, Application, Result};
use hyper::Server;
use std::net::SocketAddr;

pub async fn serve(app: Application, address: SocketAddr) -> Result<()> {
    let server = Server::bind(&address).serve(MakeService::from(app));
    let ctrlc = async {
        let message = "failed to install CTRL+C signal handler";
        tokio::signal::ctrl_c().await.expect(message);
    };

    println!("Server listening at http://{}", address);
    Ok(server.with_graceful_shutdown(ctrlc).await?)
}
