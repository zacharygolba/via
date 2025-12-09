use native_tls::{Identity, Protocol};
use std::future::Future;
use std::process::ExitCode;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_native_tls::TlsAcceptor;

use super::super::accept;
use super::super::server::ServerConfig;
use crate::app::AppService;
use crate::error::Error;

const MIN_PROTOCOL_VERSION: Protocol = Protocol::Tlsv12;

pub fn listen_native_tls<App, A>(
    config: ServerConfig,
    address: A,
    identity: Identity,
    service: AppService<App>,
) -> impl Future<Output = Result<ExitCode, Error>>
where
    A: ToSocketAddrs,
    App: Send + Sync + 'static,
{
    let handshake = {
        let acceptor = TlsAcceptor::from(
            native_tls::TlsAcceptor::builder(identity)
                .min_protocol_version(Some(MIN_PROTOCOL_VERSION))
                .build()
                .expect("failed to build native_tls::TlsAcceptor"),
        );

        Box::new(move |stream| {
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
