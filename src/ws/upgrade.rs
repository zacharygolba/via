use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use std::fmt::Display;
use std::mem::swap;
use std::ops::Deref;
use std::sync::Arc;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use super::channel::Channel;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::{Request, RequestHead};
use crate::response::Response;
use crate::{Error, raise};

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB
const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug)]
pub struct Context<State = ()>(Arc<RequestHead<State>>);

pub struct Upgrade<F> {
    config: StreamConfig,
    upgraded: Arc<F>,
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

fn handle_error<E: Display>(error: &E) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

async fn start<State, F, R>(trx: Arc<F>, config: StreamConfig, request: Request<State>)
where
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send,
{
    let (mut head, _) = request.into_parts();
    let mut request = http::Request::new(());

    swap(request.extensions_mut(), &mut head.parts.extensions);

    let context = Context(Arc::new(head));
    let stream = match hyper::upgrade::on(request).await {
        Ok(io) => config.apply(Builder::new()).serve(TokioIo::new(io)),
        Err(error) => return handle_error(&error),
    };

    tokio::pin!(stream);

    let result = 'session: loop {
        let (channel, tx, mut rx) = Channel::new();
        let mut future = Box::pin(trx(channel, context.clone()));

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

impl<State> Clone for Context<State> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<State> Deref for Context<State> {
    type Target = RequestHead<State>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
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
            Arc::clone(&self.upgraded),
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
