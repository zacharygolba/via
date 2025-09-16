use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};

use super::message::Message;
use crate::error::Error;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::{OwnedPathParams, PathParam, QueryParam, Request};
use crate::response::Response;

const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

type OnUpgrade<T> =
    dyn Fn(WebSocket, RequestContext<T>) -> BoxFuture<Result<(), Error>> + Send + Sync;

pub struct RequestContext<T> {
    params: Arc<OwnedPathParams>,
    state: Arc<T>,
}

pub struct Handshake<T> {
    max_payload_len: Option<usize>,
    flush_threshold: usize,
    frame_size: usize,
    on_upgrade: Arc<OnUpgrade<T>>,
}

pub struct WebSocket {
    sender: Sender<Message>,
    receiver: Receiver<Result<tokio_websockets::Message, tokio_websockets::Error>>,
}

pub fn ws<T, F, R>(upgraded: F) -> Handshake<T>
where
    F: Fn(WebSocket, RequestContext<T>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + Sync + 'static,
{
    let frame_size = 4 * 1024;
    let flush_threshold = frame_size * 2;

    Handshake {
        flush_threshold,
        frame_size,
        max_payload_len: None,
        on_upgrade: Arc::new(move |socket, request| Box::pin(upgraded(socket, request))),
    }
}

fn handle_error(error: &(dyn std::error::Error + 'static)) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

async fn handle_upgrade<T>(
    mut can_upgrade: http::Request<()>,
    stream_builder: Builder,
    on_upgrade: Arc<OnUpgrade<T>>,
    params: Arc<OwnedPathParams>,
    state: Arc<T>,
) {
    let stream = match hyper::upgrade::on(&mut can_upgrade).await {
        Ok(io) => stream_builder.serve(TokioIo::new(io)),
        Err(error) => return handle_error(&error),
    };

    tokio::pin!(stream);

    'session: loop {
        let (sender, mut rx) = mpsc::channel(128);
        let (tx, receiver) = mpsc::channel(128);
        let mut future = on_upgrade(
            WebSocket { sender, receiver },
            RequestContext {
                params: Arc::clone(&params),
                state: Arc::clone(&state),
            },
        );

        loop {
            tokio::select! {
                Some(message) = rx.recv() => {
                    match stream.send(message.into()).await {
                        Err(tokio_websockets::Error::AlreadyClosed) => break 'session,
                        Err(error) => handle_error(&error),
                        Ok(_) => {},
                    }
                }
                Some(next) = stream.next() => {
                    let send_result = match next {
                        Ok(message) if message.is_ping() || message.is_pong() => continue,
                        Ok(message) => tx.send(Ok(message)).await,

                        Err(tokio_websockets::Error::AlreadyClosed) => break 'session,
                        result @ Err(_) => tx.send(result).await,
                    };

                    if send_result.is_err() {
                        break 'session;
                    }
                }
                result = future.as_mut() => {
                    if let Err(error) = result {
                        handle_error(error.source());
                        continue 'session;
                    }

                    break 'session;
                },
            }
        }
    }

    if cfg!(debug_assertions) {
        println!("websocket session ended");
    }
}

fn validate_accept_key<T>(request: &Request<T>) -> Result<String, crate::Error> {
    let mut hasher = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);
    let accept_key = request
        .header(&header::SEC_WEBSOCKET_KEY)?
        .ok_or_else(|| crate::error!(400, "Missing required header: \"Sec-Websocket-Key\"."))?;

    hasher.update(accept_key.as_bytes());
    hasher.update(GUID);

    Ok(base64_engine.encode(hasher.finish()))
}

fn validate_websocket_version<T>(request: &Request<T>) -> Result<(), crate::Error> {
    match request.header(header::SEC_WEBSOCKET_VERSION)? {
        Some("13") | None => Ok(()),
        Some(version) => Err(crate::error!(
            400,
            "Unsupported websocket version: \"{}\".",
            version
        )),
    }
}

impl WebSocket {
    pub async fn send<T>(&self, message: T) -> Result<(), Error>
    where
        Message: From<T>,
    {
        if let Err(_) = self.sender.send(message.into()).await {
            Err(tokio_websockets::Error::AlreadyClosed.into())
        } else {
            Ok(())
        }
    }

    pub async fn next(&mut self) -> Result<Message, Error> {
        match self.receiver.recv().await {
            Some(Ok(next)) => {
                let message = next.try_into()?;

                if matches!(&message, Message::Close(_)) {
                    self.receiver.close();
                }

                Ok(message)
            }
            Some(Err(error)) => Err(error.into()),
            None => Err(tokio_websockets::Error::AlreadyClosed.into()),
        }
    }
}

impl<T> RequestContext<T> {
    pub fn path(&self) -> &str {
        self.params.path()
    }

    pub fn state(&self) -> &Arc<T> {
        &self.state
    }

    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.params.get(name)
    }

    pub fn query<'b>(&self, name: &'b str) -> QueryParam<'_, 'b> {
        QueryParam::new(name, self.params.query())
    }
}

impl<T> Handshake<T> {
    pub fn flush_threshold(mut self, flush_threshold: usize) -> Self {
        self.flush_threshold = flush_threshold;
        self
    }

    pub fn frame_size(mut self, frame_size: usize) -> Self {
        self.flush_threshold = frame_size;
        self
    }

    pub fn max_payload_len(mut self, max_payload_len: Option<usize>) -> Self {
        self.max_payload_len = max_payload_len;
        self
    }
}

impl<T> Handshake<T> {
    fn stream_builder(&self) -> Builder {
        Builder::new()
            .limits(Limits::default().max_payload_len(self.max_payload_len))
            .config(
                Config::default()
                    .flush_threshold(self.flush_threshold)
                    .frame_size(self.frame_size),
            )
    }
}

impl<T: Send + Sync + 'static> Middleware<T> for Handshake<T> {
    fn call(&self, request: Request<T>, next: Next<T>) -> crate::BoxFuture {
        match request.header(header::UPGRADE) {
            Ok(Some("websocket")) => {}
            Err(error) => return Box::pin(async { Err(error) }),
            Ok(_) => return next.call(request),
        }

        if let Err(error) = validate_websocket_version(&request) {
            return Box::pin(async { Err(error) });
        }

        let accept_key = match validate_accept_key(&request) {
            Ok(valid_key) => valid_key,
            Err(error) => return Box::pin(async { Err(error) }),
        };

        let (head, _) = request.into_parts();
        let params = Arc::new(OwnedPathParams::new(head.parts.uri.clone(), head.params));

        tokio::spawn(handle_upgrade(
            http::Request::from_parts(head.parts, ()),
            self.stream_builder(),
            Arc::clone(&self.on_upgrade),
            params,
            head.state,
        ));

        Box::pin(async {
            Response::build()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(header::CONNECTION, "Upgrade")
                .header(header::UPGRADE, "websocket")
                .header(header::SEC_WEBSOCKET_ACCEPT, accept_key)
                .finish()
        })
    }
}
