use native_tls::{Identity, Protocol};
use std::future::Future;
use std::io;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::time::timeout;
use tokio_native_tls::TlsAcceptor;

use crate::app::AppService;
use crate::error::{Error, ServerError};
use crate::server::{ServerConfig, accept};

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
    let handshake = Box::new({
        let acceptor = TlsAcceptor::from(
            native_tls::TlsAcceptor::builder(identity)
                .min_protocol_version(Some(MIN_PROTOCOL_VERSION))
                .build()
                .expect("failed to build native_tls::TlsAcceptor"),
        );

        move |timeout_in_seconds: Option<Duration>, tcp_stream| {
            let acceptor = acceptor.clone();

            async move {
                let duration = timeout_in_seconds.unwrap_or_default();
                match timeout(duration, acceptor.accept(tcp_stream)).await {
                    Ok(result) => result.map_err(ServerError::Tls),
                    Err(_) => Err(ServerError::Io(io::ErrorKind::TimedOut.into())),
                }
            }
        }
    });

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
