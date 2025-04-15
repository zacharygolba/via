use hyper::server::conn;
use hyper_util::rt::{TokioIo, TokioTimer};
use std::collections::VecDeque;
use std::error::Error;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};
use tokio::task::{JoinError, JoinHandle, JoinSet};
use tokio::time::{self, timeout, Instant};
use tokio::{signal, task};

#[cfg(feature = "http2")]
use hyper_util::rt::TokioExecutor;

use super::acceptor::Acceptor;
use super::server::ServerContext;
use crate::error::DynError;

type TaskResult = Result<(), DynError>;
type TaskHandle = JoinHandle<TaskResult>;

/// The amount of time in seconds we'll wait before garbage collecting
/// connection tasks that finished.
///
const GC_INTERVAL: Duration = Duration::from_secs(10);

const MILLISECOND: Duration = Duration::from_millis(1);

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*)
        }
    };
}

pub async fn accept<A, T>(
    listener: TcpListener,
    acceptor: A,
    context: &ServerContext<T>,
) -> ExitCode
where
    A: Acceptor + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    // Create a semaphore with a number of permits equal to the maximum number
    // of connections that the server can handle concurrently. If the maximum
    // number of connections is reached, we'll wait until a permit is available
    // before accepting a new connection.
    let semaphore = Arc::new(Semaphore::new(context.max_connections()));

    // Create a JoinSet to track inflight connections. We'll use this to wait for
    // all connections to close before the server exits.
    let mut connections = VecDeque::<TaskHandle>::with_capacity(4096);

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let mut shutdown_rx = {
        let (tx, rx) = watch::channel(None);
        tokio::spawn(wait_for_ctrl_c(tx));
        rx
    };

    // Start accepting incoming connections.
    let exit_code = 'accept: loop {
        // Acquire a permit from the semaphore.
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(acquired) => acquired,
            Err(_) => break 1.into(),
        };

        let service = context.make_service();

        // Accept the next stream from the tcp listener.
        let (stream, _addr) = {
            let accept = poll_fn({
                let listener = &listener;
                let connections = &mut connections;
                let mut interval = time::sleep(GC_INTERVAL);

                move |context| match listener.poll_accept(context) {
                    ready @ Poll::Ready(_) => ready,
                    pending => {
                        let mut sleep = unsafe { Pin::new_unchecked(&mut interval) };

                        if sleep.as_mut().poll(context).is_ready() {
                            log!("server is idle");
                            log!("  attempting to join finished tasks");

                            let total = join_finished(connections, context, MILLISECOND);
                            log!("    joined {} tasks", total);

                            sleep.as_mut().reset(Instant::now() + GC_INTERVAL);
                        }

                        pending
                    }
                }
            });

            tokio::select! {
                // A connection was accepted or IDLE_TIMEOUT expired.
                result = accept => match result {
                    Ok(accepted) => accepted,
                    Err(error) => {
                        eprintln!("error(listener): {}", error);
                        drop(permit);
                        continue;
                    }
                },
                // A shutdown signal was received.
                _ = shutdown_rx.changed() => {
                    drop(permit);
                    break 'accept match *shutdown_rx.borrow_and_update() {
                        Some(false) => 0.into(),
                        Some(true) | None => 1.into(),
                    }
                }
            }
        };

        connections.push_back({
            let mut acceptor = acceptor.clone();
            let mut shutdown_rx = shutdown_rx.clone();

            // TODO: Attempt to get the task size in the tls example <= 1024.
            task::spawn(async move {
                let io = match acceptor.accept(stream).await {
                    Ok(accepted) => TokioIo::new(accepted),
                    Err(error) => {
                        drop(permit);
                        return Err(error.into());
                    }
                };

                // Create a new HTTP/2 connection.
                #[cfg(feature = "http2")]
                let connection = conn::http2::Builder::new(TokioExecutor::new())
                    .timer(TokioTimer::new())
                    .serve_connection(io, service);

                // Create a new HTTP/1.1 connection.
                #[cfg(all(feature = "http1", not(feature = "http2")))]
                let connection = conn::http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .with_upgrades();

                tokio::pin!(connection);

                // Serve the connection.
                let result = tokio::select! {
                    served = &mut connection => served.map_err(|e| e.into()),
                    _ = shutdown_rx.changed() => {
                        connection.as_mut().graceful_shutdown();
                        connection.await.map_err(|e| e.into())
                    }
                };

                // Explicitly drop the semaphore permit.
                drop(permit);

                result
            })
        });

        if semaphore.available_permits() == 0 {
            log!("server at capacity. joining the next task.");
            if let Some(result) = join_next(&mut connections).await {
                joined(&result);
            }
        }
    };

    let drain_all = timeout(
        context.shutdown_timeout(),
        drain_connections(&mut connections),
    );

    if drain_all.await.is_ok() {
        exit_code
    } else {
        1.into()
    }
}

fn joined(result: &Result<TaskResult, JoinError>) {
    match result {
        // An error occurred that originates from hyper or tokio.
        Ok(Err(error)) => {
            if let Some(e) = error.downcast_ref::<hyper::Error>() {
                let is_disconnect = e.is_canceled()
                    || e.is_incomplete_message()
                    || e.source().is_some_and(|source| {
                        source
                            .downcast_ref::<std::io::Error>()
                            .is_some_and(|e| e.kind() == std::io::ErrorKind::NotConnected)
                    });

                if is_disconnect {
                    // trace!();
                } else {
                    log!("error(http): {}", e);
                }
            } else {
                log!("error(other): {}", error);
            }
        }
        // The connection task panicked.
        Err(error) if error.is_panic() => panic!("{}", error),
        Ok(_) | Err(_) => {}
    }
}

fn join_next(
    connections: &mut VecDeque<TaskHandle>,
) -> impl Future<Output = Option<Result<TaskResult, JoinError>>> + use<'_> {
    poll_fn(|context| {
        if let Some(next) = connections.front_mut() {
            if let Poll::Ready(result) = Pin::new(next).poll(context) {
                connections.pop_front();
                Poll::Ready(Some(result))
            } else {
                Poll::Pending
            }
        } else {
            Poll::Pending
        }
    })
}

fn join_finished(
    connections: &mut VecDeque<TaskHandle>,
    context: &mut Context,
    timeout: Duration,
) -> usize {
    let mut total = 0;
    let now = Instant::now();

    while let Some(mut task) = connections.pop_front() {
        if let Poll::Ready(result) = Pin::new(&mut task).poll(context) {
            joined(&result);
            total += 1;

            if now.elapsed() < timeout {
                continue;
            }
        }

        connections.push_back(task);
        break;
    }

    total
}

async fn drain_connections(connections: &mut VecDeque<TaskHandle>) {
    if cfg!(debug_assertions) {
        println!("draining {} inflight connections...", connections.len());
    }

    while let Some(connection) = connections.pop_front() {
        joined(&connection.await);
    }
}

async fn wait_for_ctrl_c(tx: watch::Sender<Option<bool>>) {
    if signal::ctrl_c().await.is_err() {
        eprintln!("unable to register the 'ctrl-c' signal.");
    } else if tx.send(Some(false)).is_err() {
        eprintln!("unable to notify connections to shutdown.");
    }
}
