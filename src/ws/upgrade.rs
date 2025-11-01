use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures_util::{SinkExt, StreamExt};
use http::{HeaderMap, HeaderValue, StatusCode, header};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::fmt::{self, Display, Formatter};
use std::mem::swap;
use std::ops::ControlFlow::{Break, Continue};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits, WebSocketStream};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use super::channel::{Channel, Message};
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::RequestHead;
use crate::response::Response;
use crate::{Error, raise};

const CONNECTION_UPGRADE_HEADER: HeaderValue = HeaderValue::from_static("upgrade");
const UPGRADE_WEBSOCKET_HEADER: HeaderValue = HeaderValue::from_static("websocket");

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug)]
pub struct Request<State = ()> {
    head: Arc<RequestHead<State>>,
}

pub struct Upgrade<F> {
    config: StreamConfig,
    upgraded: Arc<F>,
}

#[derive(Debug)]
enum ErrorKind {
    App(Error),
    ChannelClosed,
    WebSocket(tokio_websockets::Error),
}

#[derive(Clone)]
struct StreamConfig {
    max_payload_size: Option<usize>,
    flush_threshold: usize,
    frame_size: usize,
}

fn gen_accept_key(key: &[u8]) -> String {
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);

    hasher.update(key);
    hasher.update(GUID);

    base64_engine.encode(hasher.finish())
}

fn handle_error(error: &impl std::error::Error) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

async fn start<State, F, R>(trx: Arc<F>, config: StreamConfig, mut head: RequestHead<State>)
where
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send,
{
    let stream = {
        let mut request = http::Request::new(());
        swap(request.extensions_mut(), &mut head.parts.extensions);

        let result = hyper::upgrade::on(&mut request).await;
        swap(&mut head.parts.extensions, request.extensions_mut());

        match result {
            Ok(upgraded) => config.apply(TokioIo::new(upgraded)),
            Err(error) => return handle_error(&error),
        }
    };

    let head = Arc::new(head);

    tokio::pin!(stream);

    'session: loop {
        let (sender, mut rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);
        let mut future = Box::pin(trx(
            Channel::new(sender, receiver),
            Request::new(Arc::clone(&head)),
        ));

        loop {
            let flow = tokio::select! {
                // Forward the incoming message to the channel.
                Some(result) = stream.next() => {
                    match result.and_then(Message::try_from) {
                        Err(error) => Continue(Some(ErrorKind::WebSocket(error))),
                        Ok(message) => match tx.send(message).await {
                            Err(_) => Break(Some(ErrorKind::ChannelClosed)),
                            Ok(_) => Continue(None),
                        },
                    }
                }

                // Forward the outbound message to the stream.
                Some(message) = rx.recv() => {
                    let result = stream.send(message.into()).await;
                    Continue(result.err().map(ErrorKind::WebSocket))
                }

                // The future returned from app code is ready.
                result = future.as_mut() => match result {
                    Err(Continue(error)) => Continue(Some(ErrorKind::App(error))),
                    Err(Break(error)) => Break(Some(ErrorKind::App(error))),
                    Ok(_) => Break(None),
                },
            };

            match flow {
                Continue(Some(error @ ErrorKind::App(_))) => {
                    handle_error(&error);
                    continue 'session;
                }
                Continue(option) => {
                    option.inspect(handle_error);
                }
                Break(option) => {
                    option.inspect(handle_error);

                    if let Err(error) = stream.flush().await {
                        handle_error(&error);
                    }

                    break 'session;
                }
            }
        }
    }

    if cfg!(debug_assertions) {
        println!("websocket session ended");
    }
}

impl std::error::Error for ErrorKind {}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::App(error) => Display::fmt(error, f),
            Self::ChannelClosed => write!(f, ""),
            Self::WebSocket(error) => Display::fmt(error, f),
        }
    }
}

impl<State> Request<State> {
    fn new(head: Arc<RequestHead<State>>) -> Self {
        Self { head }
    }
}

impl<State> Deref for Request<State> {
    type Target = RequestHead<State>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Deref::deref(&self.head)
    }
}

impl StreamConfig {
    fn apply(self, io: TokioIo<Upgraded>) -> WebSocketStream<TokioIo<Upgraded>> {
        let limits = Limits::default().max_payload_len(self.max_payload_size);
        let config = Config::default()
            .frame_size(self.frame_size)
            .flush_threshold(self.flush_threshold);

        Builder::new().config(config).limits(limits).serve(io)
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

impl<F> Upgrade<F> {
    pub(super) fn new(upgraded: F) -> Self {
        Self {
            config: Default::default(),
            upgraded: Arc::new(upgraded),
        }
    }

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

fn is_websocket_upgrade(headers: &HeaderMap) -> bool {
    headers
        .get(header::UPGRADE)
        .is_some_and(|value| value == "websocket")
}

fn version_is_supported(headers: &HeaderMap) -> bool {
    headers
        .get(header::SEC_WEBSOCKET_VERSION)
        .is_some_and(|value| value == "13")
}

impl<State, F, R> Middleware<State> for Upgrade<F>
where
    State: Send + Sync + 'static,
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send,
{
    fn call(&self, request: crate::Request<State>, next: Next<State>) -> BoxFuture {
        let headers = request.headers();

        if !is_websocket_upgrade(headers) {
            return next.call(request);
        }

        if !version_is_supported(headers) {
            return Box::pin(async {
                raise!(400, message = "sec-websocket-version header must be \"13\"");
            });
        }

        let Some(accept) = headers
            .get(header::SEC_WEBSOCKET_KEY)
            .map(|value| gen_accept_key(value.as_ref()))
        else {
            return Box::pin(async {
                raise!(400, message = "missing required header: sec-websocket-key.")
            });
        };

        tokio::spawn({
            let trx = Arc::clone(&self.upgraded);
            let config = self.config.clone();
            let (head, _) = request.into_parts();

            Box::pin(async move { start(trx, config, head).await })
        });

        Box::pin(async {
            Response::build()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(header::CONNECTION, CONNECTION_UPGRADE_HEADER)
                .header(header::SEC_WEBSOCKET_ACCEPT, accept)
                .header(header::UPGRADE, UPGRADE_WEBSOCKET_HEADER)
                .finish()
        })
    }
}
