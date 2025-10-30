use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use bytes::{Buf, Bytes, TryGetError};
use bytestring::ByteString;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_websockets::proto::ProtocolError;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use crate::error::Error;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::payload::{self, Payload};
use crate::raise;
use crate::request::{OwnedPathParams, PathParam, QueryParam, Request};
use crate::response::Response;

pub use tokio_websockets::CloseCode;

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug)]
#[non_exhaustive]
pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteString>)>),
    Text(ByteString),
}

pub struct Channel {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

pub struct Context<State = ()> {
    params: OwnedPathParams,
    state: Arc<State>,
}

pub struct Upgrade<F> {
    config: StreamConfig,
    on_upgrade: Arc<F>,
}

#[derive(Clone)]
struct StreamConfig {
    max_payload_size: Option<usize>,
    flush_threshold: usize,
    frame_size: usize,
}

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::ws::{self, Message};
/// use via::{App, Error, Payload};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let mut app = App::new(());
///
///     // GET /echo ~> web socket upgrade.
///     app.route("/echo").to(via::ws(echo));
///
///     Ok(())
/// }
///
/// async fn echo(mut channel: ws::Channel, _: ws::Context) -> via::Result<()> {
///     while let Some(message) = channel.next().await {
///         match message {
///             echo @ (Message::Binary(_) | Message::Text(_)) => {
///                 channel.send(echo).await?;
///             }
///             Message::Close(close) => {
///                 if let Some((code, reason)) = close {
///                     eprintln!("close: code = {}, reason = {:?}", u16::from(code), reason);
///                 }
///                 break;
///             }
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
///```
///
pub fn websocket<State, F, R>(upgraded: F) -> Upgrade<F>
where
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + Sync + 'static,
{
    Upgrade {
        config: StreamConfig::default(),
        on_upgrade: Arc::new(upgraded),
    }
}

fn handle_error(error: &(dyn std::error::Error + 'static)) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

async fn start<State, F, R>(trx: Arc<F>, config: StreamConfig, request: Request<State>)
where
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send,
{
    let (stream, context) = {
        let path_and_query = request.uri().path_and_query().cloned();
        let (head, _) = request.into_parts();
        let context = Context {
            params: OwnedPathParams::new(path_and_query, head.params),
            state: head.state,
        };

        let mut request = http::Request::from_parts(head.parts, ());
        let stream = match hyper::upgrade::on(&mut request).await {
            Ok(io) => config.apply(Builder::new()).serve(TokioIo::new(io)),
            Err(error) => return handle_error(&error),
        };

        (stream, context)
    };

    tokio::pin!(stream);

    let result = 'session: loop {
        let (sender, mut rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);
        let mut future = Box::pin(trx(Channel { sender, receiver }, context.clone()));

        loop {
            let error_opt = tokio::select! {
                Some(message) = rx.recv() => stream.send(message.into()).await.err(),
                Some(result) = stream.next() => match result {
                    Err(error) => Some(error),
                    Ok(next) => {
                        if next.is_ping() || next.is_pong() {
                            continue;
                        }

                        match next.try_into() {
                            Err(error) => Some(error),
                            Ok(message) => {
                                let _ = tx.send(message).await;
                                None
                            }
                        }
                    }
                },
                result = future.as_mut() => {
                    if let Err(error) = result {
                        if cfg!(debug_assertions) {
                            eprintln!("error(ws): {}", error);
                        }
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

fn gen_accept_key(key: &[u8]) -> String {
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);

    hasher.update(key);
    hasher.update(GUID);

    base64_engine.encode(hasher.finish())
}

impl Channel {
    pub async fn send(&mut self, message: impl Into<Message>) -> Result<(), Error> {
        if self.sender.send(message.into()).await.is_err() {
            Err(tokio_websockets::Error::AlreadyClosed.into())
        } else {
            Ok(())
        }
    }

    pub async fn next(&mut self) -> Option<Message> {
        self.receiver.recv().await
    }
}

impl<State> Context<State> {
    #[inline]
    pub fn into_state(self) -> Arc<State> {
        self.state
    }

    #[inline]
    pub fn path(&self) -> &str {
        self.params.path()
    }

    #[inline]
    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.params.get(name)
    }

    #[inline]
    pub fn query<'b>(&self, name: &'b str) -> QueryParam<'_, 'b> {
        QueryParam::new(name, self.params.query())
    }
}

impl<State> Clone for Context<State> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            params: self.params.clone(),
        }
    }
}

impl From<Bytes> for Message {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<ByteString> for Message {
    #[inline]
    fn from(data: ByteString) -> Self {
        Self::Text(data)
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

impl From<String> for Message {
    #[inline]
    fn from(data: String) -> Self {
        ByteString::from(data).into()
    }
}

impl From<&'_ str> for Message {
    #[inline]
    fn from(data: &'_ str) -> Self {
        ByteString::from(data).into()
    }
}

impl Payload for Message {
    fn copy_to_bytes(self) -> Bytes {
        match self {
            Self::Binary(bytes) => Payload::copy_to_bytes(bytes),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                Payload::copy_to_bytes(utf8.into_bytes())
            }
        }
    }

    fn into_utf8(self) -> Result<String, Error> {
        match self {
            Self::Binary(bytes) => bytes.into_utf8(),
            Self::Close(None) | Self::Close(Some((_, None))) => Ok(Default::default()),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                let vec = utf8.into_bytes().into_vec();
                // Safety: ValidUtf8 is only constructed from valid UTF-8 byte sequences.
                unsafe { Ok(String::from_utf8_unchecked(vec)) }
            }
        }
    }

    fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Binary(bytes) => bytes.into_vec(),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => utf8.into_bytes().into_vec(),
        }
    }

    fn serde_json_untagged<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let detached = match self {
            Self::Binary(mut bytes) => bytes.split_to(bytes.len()),
            Self::Close(None) | Self::Close(Some((_, None))) => Bytes::new(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                let mut bytes = utf8.into_bytes();
                bytes.split_to(bytes.len())
            }
        };

        // Allocation not required when json is sourced from a ws message.
        payload::deserialize_json(detached.as_ref())
    }
}

impl TryFrom<tokio_websockets::Message> for Message {
    type Error = tokio_websockets::Error;

    fn try_from(message: tokio_websockets::Message) -> Result<Self, Self::Error> {
        let is_binary = message.is_binary();
        let is_text = !is_binary && message.is_text();

        let mut bytes = Bytes::from(message.into_payload());

        if is_binary {
            Ok(Self::Binary(bytes))
        } else if is_text {
            let utf8 = bytes.try_into().or(Err(ProtocolError::InvalidUtf8))?;
            Ok(Self::Text(utf8))
        } else {
            // Continuation, Ping, and Pong messages are handled by
            // tokio_websockets. The message opcode must be close.
            match bytes.try_get_u16() {
                // The payload is empty and therefore, valid.
                Err(TryGetError { available: 0, .. }) => Ok(Self::Close(None)),

                // The payload starts with an invalid close code.
                Ok(0..=999) | Ok(4999..) | Err(_) => Err(ProtocolError::InvalidCloseCode.into()),

                // The payload contains a valid close code and reason.
                Ok(u16) => {
                    let code = u16.try_into()?;

                    Ok(if bytes.remaining() == 0 {
                        Self::Close(Some((code, None)))
                    } else {
                        let reason = bytes.try_into().or(Err(ProtocolError::InvalidUtf8))?;
                        Self::Close(Some((code, Some(reason))))
                    })
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

impl StreamConfig {
    fn apply(self, builder: Builder) -> Builder {
        builder
            .limits(Limits::default().max_payload_len(self.max_payload_size))
            .config(
                Config::default()
                    .frame_size(self.frame_size)
                    .flush_threshold(self.flush_threshold),
            )
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            frame_size: DEFAULT_FRAME_SIZE,
            flush_threshold: DEFAULT_FRAME_SIZE,
            max_payload_size: Some(DEFAULT_FRAME_SIZE),
        }
    }
}

impl<State> Upgrade<State> {
    /// The threshold at which the bytes queued at socket are flushed.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn flush_threshold(mut self, flush_threshold: usize) -> Self {
        self.config.flush_threshold = flush_threshold;
        self
    }

    /// The frame size used for messages in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn frame_size(mut self, frame_size: usize) -> Self {
        self.config.frame_size = frame_size;
        self
    }

    /// The maximum payload size in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn max_payload_size(mut self, max_payload_size: Option<usize>) -> Self {
        self.config.max_payload_size = max_payload_size;
        self
    }
}

impl<State, F, R> Middleware<State> for Upgrade<F>
where
    State: Send + Sync + 'static,
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let upgrade = match request.headers().get(header::UPGRADE) {
            Some(value) if value == "websocket" => value.clone(),
            _ => return next.call(request),
        };

        match request.header(header::SEC_WEBSOCKET_VERSION) {
            Ok(Some("13")) => {}
            Err(error) => return Box::pin(async { Err(error) }),
            Ok(_) => {
                return Box::pin(async {
                    raise!(
                        400,
                        message = "sec-websocket-version header must be \"13\"."
                    )
                });
            }
        }

        let accept = match request.header(header::SEC_WEBSOCKET_KEY) {
            Ok(Some(key)) => gen_accept_key(key.as_bytes()),
            Err(error) => return Box::pin(async { Err(error) }),
            Ok(None) => {
                return Box::pin(async {
                    raise!(400, message = "missing required header: sec-websocket-key.")
                });
            }
        };

        tokio::spawn(Box::pin(start(
            Arc::clone(&self.on_upgrade),
            self.config.clone(),
            request,
        )));

        Box::pin(async {
            Response::build()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(header::CONNECTION, "upgrade")
                .header(header::SEC_WEBSOCKET_ACCEPT, accept)
                .header(header::UPGRADE, upgrade)
                .finish()
        })
    }
}
