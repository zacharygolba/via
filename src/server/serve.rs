use hyper::server::conn;
use hyper_util::rt::TokioTimer;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;
use tokio::{signal, time};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use crate::app::{App, AppService};
use crate::error::DynError;
use crate::server::io_stream::IoStream;

pub async fn serve<A, T>(
    listener: TcpListener,
    acceptor: A,
    app: App<T>,
    max_body_size: usize,
    max_connections: usize,
    shutdown_timeout: Duration,
) -> Result<ExitCode, DynError>
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(max_connections));

    // Wrap app in an arc so it can be cloned into the connection task.
    let app = Arc::new(app);

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = JoinSet::new();

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = semaphore.clone().acquire_owned().await?;

        // Wait for something interesting to happen.
        let tcp_stream = loop {
            tokio::select! {
                // A graceful shutdown was requested.
                _ = shutdown_rx.changed() => {
                    // Break out of the accept loop with the corrosponding exit code.
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        Some(false) => ExitCode::from(0),
                        Some(true) | None => ExitCode::from(1),
                    }
                }

                // A new connection is ready to be accepted.
                result = listener.accept() => match result {
                    // Accept the stream from the acceptor.
                    Ok((stream, _addr)) => break stream,
                    Err(error) => {
                        let _ = &error; // Placeholder for tracing...
                    }
                },

                // We have idle time. Join any inflight connections that may
                // have finished.
                _ = connections.join_next(), if !connections.is_empty() => {
                    while connections.try_join_next().is_some() {}
                }
            }
        };

        // Clone the app so it can be moved into the connection task.
        let app = Arc::clone(&app);

        // Clone acceptor so negotiation can happen in the connection task.
        let mut acceptor = acceptor.clone();

        // Clone the watch receiver so we can shutdown the connection if a
        // ctrl+c signal is sent to the process.
        let mut shutdown_rx = shutdown_rx.clone();

        // Spawn a task to serve the connection.
        connections.spawn(async move {
            let stream = match acceptor.accept(tcp_stream).await {
                Ok(accepted) => accepted,
                Err(_) => {
                    // Placeholder for tracing...
                    drop(permit);
                    return;
                }
            };

            // Create a new HTTP/2 connection.
            #[cfg(feature = "http2")]
            let mut connection = conn::http2::Builder::new(TokioExecutor::new())
                .timer(TokioTimer::new())
                .serve_connection(IoStream::new(stream), AppService::new(app, max_body_size));

            // Create a new HTTP/1.1 connection.
            #[cfg(all(feature = "http1", not(feature = "http2")))]
            let mut connection = conn::http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(IoStream::new(stream), AppService::new(app, max_body_size))
                .with_upgrades();

            let mut conn_mut = Pin::new(&mut connection);

            // Serve the connection.
            if let Err(_) = tokio::select!(
                result = conn_mut.as_mut() => result,
                _ = shutdown_rx.changed() => {
                    conn_mut.as_mut().graceful_shutdown();
                    conn_mut.as_mut().await
                }
            ) {
                // Placeholder for tracing...
            }

            drop(permit);
        });
    };

    tokio::select! {
        // Wait for inflight connection to close within the configured timeout.
        _ = shutdown(&mut connections) => Ok(exit_code),

        // Otherwise, return an error.
        _ = time::sleep(shutdown_timeout) => {
            Err("server exited before all connections were closed".into())
        }
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}

async fn shutdown(connections: &mut JoinSet<()>) {
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            let _ = &error; // Placeholder for tracing...
        }
    }
}
