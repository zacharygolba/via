use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use bytes::{Buf, Bytes, TryGetError};
use bytestring::ByteString;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_websockets::proto::ProtocolError;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use crate::Payload;
use crate::error::Error;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::{OwnedPathParams, PathParam, QueryParam, Request};
use crate::response::Response;
use crate::response::body::STANDARD_FRAME_LEN;

pub use tokio_websockets::CloseCode;

const DEFAULT_FRAME_SIZE: usize = 1024 * 4; // 4 KB
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

type OnUpgrade<State> =
    dyn Fn(WebSocket, Context<State>) -> BoxFuture<Result<(), Error>> + Send + Sync;

#[derive(Debug)]
#[non_exhaustive]
pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteString>)>),
    Text(ByteString),
}

pub struct Context<State = ()> {
    params: OwnedPathParams,
    state: Arc<State>,
}

pub struct Upgrade<State> {
    max_payload_size: Option<usize>,
    flush_threshold: usize,
    frame_size: usize,
    on_upgrade: Arc<OnUpgrade<State>>,
}

pub struct WebSocket {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::{App, BoxError, Payload};
/// use via::builtin::ws::{self, Message, WebSocket};
///
/// #[tokio::main]
/// async fn main() -> Result<(), BoxError> {
///     let mut app = App::new(());
///
///     // GET /echo ~> web socket upgrade.
///     app.at("/echo").respond(via::ws(echo));
/// }
///
/// async fn echo(mut socket: WebSocket, _: ws::Context) -> via::Result<()> {
///     while let Some(message) = socket.next().await {
///         if matches!(&message, Message::Close(_)) {
///             message.as_str().inspect(|reason| eprintln!("close: {}", reason));
///             break;
///         }
///
///         socket.send(message.into_vec()).await?;
///     }
///
///     Ok(())
/// }
///```
///
pub fn ws<State, F, R>(upgraded: F) -> Upgrade<State>
where
    F: Fn(WebSocket, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + Sync + 'static,
{
    Upgrade {
        flush_threshold: STANDARD_FRAME_LEN,
        frame_size: DEFAULT_FRAME_SIZE,
        max_payload_size: Some(STANDARD_FRAME_LEN),
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
    context: Context<T>,
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
                Some(message) = rx.recv() => stream.send(message.into()).await.err(),
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
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);
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

impl<State> Context<State> {
    #[inline]
    pub fn path(&self) -> &str {
        self.params.path()
    }

    #[inline]
    pub fn state(&self) -> &State {
        &self.state
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

impl Payload for Message {
    #[inline]
    fn as_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Close(Some((_, Some(bytestring)))) | Self::Text(bytestring) => {
                Payload::as_slice(bytestring)
            }
            Self::Binary(bytes) => Payload::as_slice(bytes),
            _ => None,
        }
    }

    #[inline]
    fn as_str(&self) -> Result<Option<&str>, Error> {
        match self {
            Self::Close(Some((_, Some(bytestring)))) | Self::Text(bytestring) => {
                Payload::as_str(bytestring)
            }
            Self::Binary(bytes) => Payload::as_str(bytes),
            _ => Ok(None),
        }
    }

    #[inline]
    fn into_utf8(self) -> Result<String, Error> {
        match self {
            Self::Close(Some((_, Some(bytestring)))) | Self::Text(bytestring) => {
                Payload::into_utf8(bytestring)
            }
            Self::Binary(bytes) => Payload::into_utf8(bytes),
            _ => Ok(Default::default()),
        }
    }

    #[inline]
    fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Close(Some((_, Some(text)))) | Self::Text(text) => {
                Payload::into_vec(text.into_bytes())
            }
            Self::Binary(bytes) => Payload::into_vec(bytes),
            _ => Default::default(),
        }
    }
}

impl Payload for ByteString {
    #[inline]
    fn as_slice(&self) -> Option<&[u8]> {
        Some(self.as_ref())
    }

    #[inline]
    fn as_str(&self) -> Result<Option<&str>, Error> {
        Ok(self.as_slice().map(|slice| {
            // Safety: self is guaranteed to be valid UTF-8.
            unsafe { str::from_utf8_unchecked(slice) }
        }))
    }

    #[inline]
    fn into_utf8(self) -> Result<String, Error> {
        // Safety: self is guaranteed to be valid UTF-8.
        Ok(unsafe { String::from_utf8_unchecked(self.into_vec()) })
    }

    #[inline]
    fn into_vec(self) -> Vec<u8> {
        self.into_bytes().into_vec()
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
            } else {
                // Continuation, Ping, and Pong messages are handled by
                // tokio_websockets. The message opcode must be close.
                match bytes.try_get_u16() {
                    // The payload is empty and therefore, valid.
                    Err(TryGetError { available: 0, .. }) => Ok(Self::Close(None)),

                    // The payload starts with an invalid close code.
                    Ok(0..=999) | Ok(4999..) | Err(_) => {
                        Err(ProtocolError::InvalidCloseCode.into())
                    }

                    // The payload contains a valid close code and reason.
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

impl<State> Upgrade<State> {
    /// The threshold at which the bytes queued at socket are flushed.
    ///
    /// **Default:** `8 KB`
    ///
    pub fn flush_threshold(mut self, flush_threshold: usize) -> Self {
        self.flush_threshold = flush_threshold;
        self
    }

    /// The frame size used for messages in bytes.
    ///
    /// **Default:** `4 KB`
    ///
    pub fn frame_size(mut self, frame_size: usize) -> Self {
        self.frame_size = frame_size;
        self
    }

    /// The maximum payload size in bytes.
    ///
    /// **Default:** `8 KB`
    ///
    pub fn max_payload_size(mut self, max_payload_size: Option<usize>) -> Self {
        self.max_payload_size = max_payload_size;
        self
    }
}

impl<State> Upgrade<State> {
    fn stream_builder(&self) -> Builder {
        Builder::new()
            .limits(Limits::default().max_payload_len(self.max_payload_size))
            .config(
                Config::default()
                    .flush_threshold(self.flush_threshold)
                    .frame_size(self.frame_size),
            )
    }
}

impl<State> Middleware<State> for Upgrade<State>
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> crate::BoxFuture {
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
        let context = Context {
            params: OwnedPathParams::new(head.uri().path_and_query().cloned(), head.params),
            state: head.state,
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

impl WebSocket {
    pub async fn send(&self, message: impl Into<Message>) -> Result<(), Error> {
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
