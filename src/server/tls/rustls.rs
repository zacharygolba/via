use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_rustls::server::TlsAcceptor;

use super::super::accept;
use super::super::server::ServerConfig;
use crate::app::AppService;
use crate::error::BoxError;

pub fn listen_rustls<State, A>(
    config: ServerConfig,
    address: A,
    tls_config: rustls::ServerConfig,
    service: AppService<State>,
) -> impl Future<Output = Result<ExitCode, BoxError>>
where
    A: ToSocketAddrs,
    State: Send + Sync + 'static,
{
    let handshake = {
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Arc::new(move |stream| {
            let acceptor = acceptor.clone();
            async move { Ok(acceptor.accept(stream).await?) }
        })
    };

    async {
        let exit = accept(
            config,
            TcpListener::bind(address).await?,
            handshake,
            service,
        );

        Ok(exit.await)
    }
}
