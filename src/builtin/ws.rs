use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use bytes::{Buf, Bytes};
use bytestring::ByteString;
use futures::{SinkExt, StreamExt};
use http::{StatusCode, Uri, header};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_websockets::proto::ProtocolError;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context, SHA1_FOR_LEGACY_USE_ONLY};

use crate::error::Error;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::{OwnedPathParams, PathParam, QueryParam, Request};
use crate::response::Response;

pub use tokio_websockets::CloseCode;

const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

type OnUpgrade<T> =
    dyn Fn(WebSocket, RequestContext<T>) -> BoxFuture<Result<(), Error>> + Send + Sync;

#[derive(Debug)]
pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteString>)>),
    Text(ByteString),
}

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
    receiver: Receiver<Message>,
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
    context: RequestContext<T>,
) {
    let stream = match hyper::upgrade::on(&mut can_upgrade).await {
        Ok(io) => stream_builder.serve(TokioIo::new(io)),
        Err(error) => return handle_error(&error),
    };

    tokio::pin!(stream);

    let result = 'session: loop {
        let (sender, mut rx) = mpsc::channel(128);
        let (tx, receiver) = mpsc::channel(128);
        let mut future = on_upgrade(WebSocket { sender, receiver }, context.clone());

        loop {
            let error_opt = tokio::select! {
                Some(message) = rx.recv() => match stream.send(message.into()).await {
                    Err(error) => Some(error),
                    Ok(_) => None,
                },
                Some(result) = stream.next() => match result {
                    Ok(next) if next.is_ping() || next.is_pong() => continue,

                    Err(error) => Some(error),
                    Ok(next) => match next.try_into() {
                        Err(error) => Some(error),
                        Ok(message) => {
                            let _ = tx.send(message).await;
                            None
                        }
                    },
                },
                result = future.as_mut() => {
                    if let Err(error) = result {
                        handle_error(error.source());
                        continue 'session;
                    }

                    break 'session stream.flush().await;
                },
            };

            if let Some(error) = &error_opt {
                handle_error(error);
            }
        }
    };

    if let Err(error) = &result {
        handle_error(error);
    }

    if cfg!(debug_assertions) {
        println!("websocket session ended");
    }
}

fn validate_accept_key<T>(request: &Request<T>) -> Result<String, crate::Error> {
    let mut hasher = Context::new(&SHA1_FOR_LEGACY_USE_ONLY);
    let accept_key = request
        .header(&header::SEC_WEBSOCKET_KEY)?
        .ok_or_else(|| crate::error!(400, "missing required header: \"Sec-Websocket-Key\""))?;

    hasher.update(accept_key.as_bytes());
    hasher.update(GUID);

    Ok(base64_engine.encode(hasher.finish()))
}

#[inline]
fn validate_utf8(bytes: Bytes) -> Result<ByteString, ProtocolError> {
    bytes.try_into().or(Err(ProtocolError::InvalidUtf8))
}

fn validate_websocket_version<T>(request: &Request<T>) -> Result<(), crate::Error> {
    match request.header(header::SEC_WEBSOCKET_VERSION)? {
        Some("13") => Ok(()),
        Some(_) | None => Err(crate::error!(400, "unsupported websocket version")),
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
        let context = RequestContext {
            state: head.state,
            params: Arc::new(OwnedPathParams::new(head.parts.uri.clone(), head.params)),
        };

        tokio::spawn(handle_upgrade(
            http::Request::from_parts(head.parts, ()),
            self.stream_builder(),
            Arc::clone(&self.on_upgrade),
            context,
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

impl From<Bytes> for Message {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<Vec<u8>> for Message {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::from(Bytes::from(data))
    }
}

impl From<&'_ [u8]> for Message {
    #[inline]
    fn from(data: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(data))
    }
}

impl From<ByteString> for Message {
    #[inline]
    fn from(data: ByteString) -> Self {
        Self::Text(data)
    }
}

impl From<String> for Message {
    #[inline]
    fn from(data: String) -> Self {
        Self::from(ByteString::from(data))
    }
}

impl From<&'_ str> for Message {
    #[inline]
    fn from(data: &'_ str) -> Self {
        Self::from(ByteString::from(data))
    }
}

impl TryFrom<tokio_websockets::Message> for Message {
    type Error = tokio_websockets::Error;

    fn try_from(message: tokio_websockets::Message) -> Result<Self, Self::Error> {
        if message.is_binary() {
            Ok(Self::Binary(message.into_payload().into()))
        } else {
            let is_text = message.is_text();
            let mut bytes = Bytes::from(message.into_payload());

            if is_text {
                Ok(Self::Text(validate_utf8(bytes)?))
            } else if bytes.is_empty() {
                Ok(Self::Close(None))
            } else {
                match bytes.try_get_u16() {
                    Ok(0..=999) | Ok(4999..) | Err(_) => {
                        Err(ProtocolError::InvalidCloseCode.into())
                    }
                    Ok(u16) => {
                        let code = u16.try_into()?;

                        Ok(if bytes.remaining() == 0 {
                            Self::Close(Some((code, None)))
                        } else {
                            let reason = validate_utf8(bytes)?;
                            Self::Close(Some((code, Some(reason))))
                        })
                    }
                }
            }
        }
    }
}

impl From<Message> for tokio_websockets::Message {
    #[inline]
    fn from(message: Message) -> Self {
        match message {
            Message::Binary(binary) => Self::binary(binary),
            Message::Text(text) => Self::text(text.into_bytes()),

            Message::Close(None) => Self::close(None, ""),
            Message::Close(Some((code, reason))) => {
                Self::close(Some(code), reason.as_deref().unwrap_or_default())
            }
        }
    }
}

impl<T> RequestContext<T> {
    #[inline]
    pub fn uri(&self) -> &Uri {
        self.params.uri()
    }

    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.params.get(name)
    }

    pub fn query<'b>(&self, name: &'b str) -> QueryParam<'_, 'b> {
        QueryParam::new(name, self.uri().query().unwrap_or_default())
    }

    #[inline]
    pub fn state(&self) -> &T {
        &self.state
    }
}

impl<T> Clone for RequestContext<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            params: Arc::clone(&self.params),
        }
    }
}

impl WebSocket {
    pub async fn send(&self, message: impl Into<Message>) -> Result<(), Error> {
        if let Err(_) = self.sender.send(message.into()).await {
            Err(tokio_websockets::Error::AlreadyClosed.into())
        } else {
            Ok(())
        }
    }

    pub async fn next(&mut self) -> Option<Message> {
        self.receiver.recv().await
    }
}
