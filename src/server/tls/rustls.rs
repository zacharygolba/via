use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::time::{Timeout, timeout};
use tokio_rustls::server::{Accept, TlsAcceptor, TlsStream};

use crate::app::AppService;
use crate::error::Error;
use crate::server::{ServerConfig, accept};

enum Negotiate {
    Ready(TlsStream<TcpStream>),
    Pending(Timeout<Accept<TcpStream>>),
}

enum NegotiateProj<'a> {
    Ready(Pin<&'a mut TlsStream<TcpStream>>),
    Pending(Pin<&'a mut Timeout<Accept<TcpStream>>>),
}

pub fn listen_rustls<App, A>(
    config: ServerConfig,
    address: A,
    tls_config: rustls::ServerConfig,
    service: AppService<App>,
) -> impl Future<Output = Result<ExitCode, Error>>
where
    A: ToSocketAddrs,
    App: Send + Sync + 'static,
{
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let handshake = Box::new(move |timeout_in_seconds: Option<Duration>, tcp_stream| {
        let mut stream = Box::pin(Negotiate::Pending(timeout(
            timeout_in_seconds.unwrap_or_default(),
            acceptor.accept(tcp_stream),
        )));

        async {
            stream.as_mut().await?;
            Ok(stream)
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

impl Negotiate {
    #[inline]
    fn map_ready<F, R>(self: Pin<&mut Self>, mut f: F) -> Poll<R>
    where
        F: FnMut(Pin<&mut TlsStream<TcpStream>>) -> Poll<R>,
    {
        match self.project() {
            NegotiateProj::Ready(stream) => f(stream),
            NegotiateProj::Pending(_) => Poll::Pending,
        }
    }

    #[inline]
    fn project(self: Pin<&mut Self>) -> NegotiateProj<'_> {
        // Safety: A pin projection. Data contained in self is never moved.
        unsafe {
            match self.get_unchecked_mut() {
                Self::Pending(handshake) => NegotiateProj::Pending(Pin::new_unchecked(handshake)),
                Self::Ready(stream) => NegotiateProj::Ready(Pin::new_unchecked(stream)),
            }
        }
    }
}

impl AsyncRead for Negotiate {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.map_ready(|stream| stream.poll_read(context, buf))
    }
}

impl AsyncWrite for Negotiate {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.map_ready(|stream| stream.poll_write(context, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        self.map_ready(|stream| stream.poll_flush(context))
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        self.map_ready(|stream| stream.poll_shutdown(context))
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context,
        bufs: &[io::IoSlice],
    ) -> Poll<io::Result<usize>> {
        self.map_ready(|stream| stream.poll_write_vectored(context, bufs))
    }
}

impl Future for Negotiate {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        if let NegotiateProj::Pending(accept) = self.as_mut().project() {
            let stream = match ready!(accept.poll(context)) {
                Ok(Ok(accepted)) => accepted,
                Ok(Err(error)) => return Poll::Ready(Err(error)),
                Err(_) => return Poll::Ready(Err(io::ErrorKind::TimedOut.into())),
            };

            unsafe {
                *self.get_unchecked_mut() = Self::Ready(stream);
            }
        }

        Poll::Ready(Ok(()))
    }
}
