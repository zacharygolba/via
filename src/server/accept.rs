use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::time;

use super::Backoff;
use crate::Error;

pub async fn accept(
    backoff: &mut Backoff,
    listener: &TcpListener,
    semaphore: &Arc<Semaphore>,
) -> Result<(TcpStream, SocketAddr, OwnedSemaphorePermit), Error> {
    loop {
        let permit = match Arc::clone(semaphore).try_acquire_owned() {
            Err(TryAcquireError::NoPermits) => {
                let delay = backoff.next();
                time::sleep(delay).await;
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
                eprintln!("failed to accept incoming connection: {}", error);
                drop(permit);
                continue;
            }
        };
    }
}
