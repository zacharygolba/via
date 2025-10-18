use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use bytes::{Buf, Bytes, TryGetError};
use bytestring::ByteString;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use std::ops::Deref;
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

pub use tokio_websockets::CloseCode;

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

type OnUpgrade<State> =
    dyn Fn(Channel, Context<State>) -> BoxFuture<Result<(), Error>> + Send + Sync;

#[derive(Debug)]
#[non_exhaustive]
pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ValidUtf8>)>),
    Text(ValidUtf8),
}

pub struct Channel {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
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

#[derive(Debug)]
pub struct ValidUtf8 {
    bytes: Bytes,
}

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::ws::{self, Message};
/// use via::{App, BoxError, Payload};
///
/// #[tokio::main]
/// async fn main() -> Result<(), BoxError> {
///     let mut app = App::new(());
///
///     // GET /echo ~> web socket upgrade.
///     app.route("/echo").respond(via::ws(echo));
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
pub fn ws<State, F, R>(upgraded: F) -> Upgrade<State>
where
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + Sync + 'static,
{
    Upgrade {
        frame_size: DEFAULT_FRAME_SIZE,
        flush_threshold: DEFAULT_FRAME_SIZE,
        max_payload_size: Some(DEFAULT_FRAME_SIZE),
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
        let mut future = on_upgrade(Channel { sender, receiver }, context.clone());

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

fn validate_accept_key<T>(request: &Request<T>) -> Result<String, crate::Error> {
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);
    let accept_key = request.header(&header::SEC_WEBSOCKET_KEY)?.ok_or_else(|| {
        crate::raise!(
            400,
            message = "Missing required header: \"Sec-Websocket-Key\"."
        )
    })?;

    hasher.update(accept_key.as_bytes());
    hasher.update(GUID);

    Ok(base64_engine.encode(hasher.finish()))
}

#[inline]
fn validate_utf8(bytes: Bytes) -> Result<ValidUtf8, ProtocolError> {
    if str::from_utf8(bytes.as_ref()).is_ok() {
        Ok(ValidUtf8 { bytes })
    } else {
        Err(ProtocolError::InvalidUtf8)
    }
}

fn validate_websocket_version<T>(request: &Request<T>) -> Result<(), crate::Error> {
    match request.header(header::SEC_WEBSOCKET_VERSION)? {
        Some("13") => Ok(()),
        Some(_) | None => Err(crate::raise!(
            400,
            message = "Unsupported websocket version."
        )),
    }
}

impl Channel {
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

impl From<ByteString> for Message {
    #[inline]
    fn from(data: ByteString) -> Self {
        Self::Text(ValidUtf8 {
            bytes: data.into_bytes(),
        })
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
        Self::Text(ValidUtf8 {
            bytes: Bytes::from(data),
        })
    }
}

impl From<&'_ str> for Message {
    #[inline]
    fn from(data: &'_ str) -> Self {
        Self::Text(ValidUtf8 {
            bytes: Bytes::copy_from_slice(data.as_bytes()),
        })
    }
}

impl Payload for Message {
    fn copy_to_bytes(&mut self) -> Bytes {
        match self {
            Self::Binary(bytes) => Payload::copy_to_bytes(bytes),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                Payload::copy_to_bytes(&mut utf8.bytes)
            }
        }
    }

    fn to_utf8(&mut self) -> Result<String, Error> {
        match self {
            Self::Binary(bytes) => bytes.to_utf8(),
            Self::Close(None) | Self::Close(Some((_, None))) => Ok(Default::default()),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                let vec = utf8.bytes.to_vec();
                // Safety: ValidUtf8 is only constructed from valid UTF-8 byte sequences.
                unsafe { Ok(String::from_utf8_unchecked(vec)) }
            }
        }
    }

    fn to_vec(&mut self) -> Vec<u8> {
        match self {
            Self::Binary(bytes) => bytes.to_vec(),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => utf8.bytes.to_vec(),
        }
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
            Ok(Self::Text(validate_utf8(bytes)?))
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
                        let reason = validate_utf8(bytes)?;
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
            Message::Text(text) => Self::text(text.bytes),

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
    /// **Default:** `16 KB`
    ///
    pub fn flush_threshold(mut self, flush_threshold: usize) -> Self {
        self.flush_threshold = flush_threshold;
        self
    }

    /// The frame size used for messages in bytes.
    ///
    /// **Default:** `16 KB`
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
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
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

impl Deref for ValidUtf8 {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        let utf8 = self.bytes.as_ref();
        // Safety: ValidUtf8 is only constructed from valid UTF-8 byte sequences.
        unsafe { str::from_utf8_unchecked(utf8) }
    }
}
