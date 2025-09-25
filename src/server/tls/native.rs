use native_tls::{Identity, Protocol};
use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio_native_tls::TlsAcceptor;

use super::super::accept;
use super::super::server::ServerConfig;
use crate::app::AppService;
use crate::error::BoxError;

#[cfg(feature = "http2")]
const MIN_PROTOCOL_VERSION: Protocol = Protocol::Tlsv12;

#[cfg(not(feature = "http2"))]
const MIN_PROTOCOL_VERSION: Protocol = Protocol::Tlsv10;

pub fn listen_native_tls<State, A>(
    config: ServerConfig,
    address: A,
    identity: Identity,
    service: AppService<State>,
) -> impl Future<Output = Result<ExitCode, BoxError>>
where
    A: ToSocketAddrs,
    State: Send + Sync + 'static,
{
    let handshake = {
        let acceptor = TlsAcceptor::from(
            native_tls::TlsAcceptor::builder(identity)
                .min_protocol_version(Some(MIN_PROTOCOL_VERSION))
                .build()
                .expect("failed to build native_tls::TlsAcceptor"),
        );

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
