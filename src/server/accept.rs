use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::time;

use super::Backoff;
use crate::Error;

const ONE_SECOND: Duration = Duration::from_secs(1);

/// Sleeps for provided `duration` or until the `wake` fn returns `true`.
async fn sleep(duration: Duration, wake: impl Fn() -> bool) {
    let mut remaining = duration;

    if remaining.as_secs() == 0 {
        time::sleep(remaining).await;
        return;
    }

    while !wake() && !remaining.is_zero() {
        if remaining.as_secs() > 0 {
            time::sleep(ONE_SECOND).await;
            remaining -= ONE_SECOND;
        } else {
            time::sleep(remaining).await;
            remaining = Duration::ZERO;
        }
    }
}

pub async fn accept(
    backoff: &mut Backoff,
    listener: &TcpListener,
    semaphore: &Arc<Semaphore>,
) -> Result<(TcpStream, SocketAddr, OwnedSemaphorePermit), Error> {
    loop {
        let permit = match Arc::clone(semaphore).try_acquire_many_owned(2) {
            Err(TryAcquireError::NoPermits) => {
                sleep(backoff.next(), || semaphore.available_permits() > 3).await;
                continue;
            }
            Err(TryAcquireError::Closed) => {
                let message = "server is not running".to_owned();
                return Err(Error::new(message));
            }
            Ok(permit) => {
                backoff.reset();
                permit
            }
        };

        return match listener.accept().await {
            Ok((stream, address)) => Ok((stream, address, permit)),
            Err(error) => {
                if cfg!(debug_assertions) {
                    eprintln!("failed to accept incoming connection: {}", error);
                }

                drop(permit);
                continue;
            }
        };
    }
}
