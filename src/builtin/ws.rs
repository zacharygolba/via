use futures::{SinkExt, StreamExt};
use http::StatusCode;
use http::header::{CONNECTION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE};
use hyper::rt::{Read, Write};
use hyper::upgrade::Upgraded;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

use crate::Response;
use crate::error::BoxError;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::{Request, RequestHead};

pub use tokio::sync::mpsc::Sender;
pub use tokio_websockets::{Message, Payload};

pub struct Ws<T> {
    config: Option<Config>,
    limits: Option<Limits>,
    receive: Arc<T>,
}

struct IoStream {
    upgraded: Upgraded,
}

pub fn ws<T, R, F>(receive: R) -> Ws<R>
where
    T: Send + Sync + 'static,
    R: Fn(&Arc<T>, &Sender<Message>, Message) -> F + Send + Sync,
    F: Future<Output = Result<(), BoxError>> + Send + Sync + 'static,
{
    Ws {
        config: None,
        limits: None,
        receive: Arc::new(receive),
    }
}

async fn upgrade<T, R, F>(
    receive: Arc<R>,
    request: Request<T>,
    builder: Builder,
) -> Result<(), BoxError>
where
    T: Send + Sync + 'static,
    R: Fn(&Arc<T>, &Sender<Message>, Message) -> F + Send + Sync,
    F: Future<Output = Result<(), BoxError>> + Send + Sync + 'static,
{
    let (RequestHead { parts, state, .. }, body) = request.into_parts();
    let (tx, mut rx) = mpsc::channel(128);
    let mut request = http::Request::from_parts(parts, body);
    let mut ws = builder.serve(IoStream::new(hyper::upgrade::on(&mut request).await?));

    loop {
        tokio::select! {
            Some(message) = rx.recv() => {
                if let Err(_) = ws.send(message).await {
                    // TODO
                }
            }
            Some(result) = ws.next() => match result {
                Ok(message) => {
                    tokio::spawn(receive(&state, &tx, message));
                },
                Err(_) => {
                    // TODO
                }
            },
            else => break Ok(()),
        }
    }
}

impl<T, R, F> Middleware<T> for Ws<R>
where
    T: Send + Sync + 'static,
    R: Fn(&Arc<T>, &Sender<Message>, Message) -> F + Send + Sync + 'static,
    F: Future<Output = Result<(), BoxError>> + Send + Sync + 'static,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        if request
            .headers()
            .get(UPGRADE)
            .is_some_and(|upgrade| b"websocket" == upgrade.as_bytes())
        {
            let receive = self.receive.clone();
            let builder = Builder::new()
                .limits(self.limits.unwrap_or_default())
                .config(self.config.unwrap_or_default());

            tokio::spawn(async {
                if let Err(error) = upgrade(receive, request, builder).await {
                    eprintln!("error(upgrade): {}", error);
                }
            });

            return Box::pin(async {
                Response::build()
                    .status(StatusCode::SWITCHING_PROTOCOLS)
                    .header(CONNECTION, "upgrade")
                    .header(UPGRADE, "websocket")
                    .header(SEC_WEBSOCKET_KEY, "")
                    .header(SEC_WEBSOCKET_VERSION, "13")
                    .finish()
            });
        }

        next.call(request)
    }
}

impl IoStream {
    fn new(upgraded: Upgraded) -> Self {
        Self { upgraded }
    }
}

impl IoStream {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Upgraded> {
        unsafe { self.map_unchecked_mut(|io| &mut io.upgraded) }
    }
}

impl AsyncRead for IoStream {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        todo!()
    }
}

impl AsyncWrite for IoStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        Write::poll_write(self.project(), cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Write::poll_flush(self.project(), cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Write::poll_shutdown(self.project(), cx)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        Write::poll_write_vectored(self.project(), cx, bufs)
    }
}
