use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{self, TcpStream, ToSocketAddrs};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::time;

use super::Backoff;
use crate::Error;

#[cfg(target_os = "macos")]
const MAX_NUM_CONNECTIONS: usize = 256;

#[cfg(not(target_os = "macos"))]
const MAX_NUM_CONNECTIONS: usize = 1024;

/// This value is subtracted from the maximum number of connections to allow
/// for a margin of error when checking the semaphore for available permits.
const SAFETY_MARGIN: usize = 8;

pub struct TcpListener {
    backoff: Backoff,
    listener: net::TcpListener,
    semaphore: Arc<Semaphore>,
}

impl TcpListener {
    pub async fn bind<T>(address: T, max_connections: Option<usize>) -> Result<Self, Error>
    where
        T: ToSocketAddrs,
    {
        let backoff = Backoff::new(2, 5);
        let listener = net::TcpListener::bind(address).await?;
        let semaphore = Arc::new(Semaphore::new(
            max_connections.unwrap_or(MAX_NUM_CONNECTIONS) - SAFETY_MARGIN,
        ));

        Ok(Self {
            backoff,
            listener,
            semaphore,
        })
    }

    pub fn local_address(&self) -> Result<SocketAddr, Error> {
        Ok(self.listener.local_addr()?)
    }

    pub async fn accept(&mut self) -> Result<(TcpStream, SocketAddr, OwnedSemaphorePermit), Error> {
        loop {
            let permit = match Arc::clone(&self.semaphore).try_acquire_owned() {
                Err(TryAcquireError::NoPermits) => {
                    let delay = self.backoff.next();
                    time::sleep(delay).await;
                    continue;
                }
                Err(TryAcquireError::Closed) => {
                    let message = "server is not running".to_owned();
                    return Err(Error::new(message));
                }
                Ok(permit) => {
                    self.backoff.reset();
                    permit
                }
            };

            return match self.listener.accept().await {
                Ok((stream, address)) => Ok((stream, address, permit)),
                Err(error) => {
                    eprintln!("failed to accept incoming connection: {}", error);
                    drop(permit);
                    continue;
                }
            };
        }
    }
}
