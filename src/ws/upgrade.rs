use base64::engine::{Engine, general_purpose::STANDARD as base64};
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::mem;
use std::ops::ControlFlow;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::coop;
use tungstenite::protocol::{Role, WebSocketConfig};

use super::channel::Channel;
use super::error::{WebSocketError, is_recoverable};
use crate::{BoxFuture, Error, Middleware, Next, Response, raise};

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB
const WS_ACCEPT_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

type WebSocketStream = tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>;

#[derive(Debug)]
pub struct Request<App>(Arc<crate::Request<App>>);

pub struct Ws<F> {
    listen: Arc<F>,
    config: WebSocketConfig,
}

macro_rules! warn {
    (#[$ctx:meta], $($arg:tt)*) => {
        eprint!("warn(ws: {}) = ", stringify!($ctx));
        eprintln!($($arg)*);
    };
}

fn gen_accept_key(key: &[u8]) -> String {
    #[cfg(feature = "aws-lc-rs")]
    let mut hasher = aws_lc_rs::digest::Context::new(&aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY);

    #[cfg(feature = "ring")]
    let mut hasher = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);

    hasher.update(key);
    hasher.update(WS_ACCEPT_GUID);
    base64.encode(hasher.finish())
}

async fn start<App, F, R>(
    mut stream: Pin<Box<WebSocketStream>>,
    listener: Arc<F>,
    context: Request<App>,
) -> crate::Result<()>
where
    F: Fn(Channel, Request<App>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send,
{
    'run: loop {
        let (sender, mut rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);
        let mut listen = Box::pin(listener(Channel::new(sender, receiver), context.clone()));

        loop {
            tokio::select! {
                biased;

                // The future returned from app code is ready.
                result = &mut listen => {
                    return match result {
                        Ok(_) => Ok(()),
                        Err(ControlFlow::Break(fatal)) => Err(fatal),
                        Err(ControlFlow::Continue(recoverable)) => {
                            warn!(#[listener], "{}", recoverable);
                            continue 'run;
                        }
                    };
                }

                // Forward the outbound message to the stream.
                Some(next) = coop::unconstrained(rx.recv()) => {
                    coop::consume_budget().await;
                    if let Err(error) = stream.feed(next).await {
                        if is_recoverable(&error) {
                            warn!(#[send], "{}", error);
                        } else {
                            return Err(error.into());
                        }
                    }
                }

                // Forward the incoming message to the channel.
                Some(result) = stream.next() => {
                    match result {
                        Ok(next) => {
                            if tx.send(next).await.is_err() {
                                return Err(WebSocketError::AlreadyClosed.into());
                            }
                        }
                        Err(error) => {
                            if is_recoverable(&error) {
                                warn!(#[recv], "{}", error);
                            } else {
                                return Err(error.into());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[inline]
async fn upgrade(
    config: WebSocketConfig,
    request: &mut http::Request<()>,
) -> Result<WebSocketStream, Error> {
    let upgraded = TokioIo::new(hyper::upgrade::on(request).await?);
    Ok(WebSocketStream::from_raw_socket(upgraded, Role::Server, Some(config)).await)
}

impl<App> Clone for Request<App> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<App> Deref for Request<App> {
    type Target = crate::Request<App>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F> Ws<F> {
    pub(super) fn new(upgraded: F) -> Self {
        Self {
            listen: Arc::new(upgraded),
            config: WebSocketConfig::default()
                .accept_unmasked_frames(false)
                .read_buffer_size(DEFAULT_FRAME_SIZE)
                .write_buffer_size(0)
                .max_write_buffer_size(DEFAULT_FRAME_SIZE)
                .max_frame_size(Some(DEFAULT_FRAME_SIZE))
                .max_message_size(Some(DEFAULT_FRAME_SIZE)),
        }
    }

    /// The frame size used for messages in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn frame_size(self, frame_size: usize) -> Self {
        Self {
            config: self.config.max_frame_size(Some(frame_size)),
            ..self
        }
    }

    /// The maximum payload size in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn max_payload_size(self, max_payload_size: Option<usize>) -> Self {
        Self {
            config: self.config.max_message_size(max_payload_size),
            ..self
        }
    }
}

impl<T, Await, App> Middleware<App> for Ws<T>
where
    T: Fn(Channel, Request<App>) -> Await + Send + Sync + 'static,
    Await: Future<Output = super::Result> + Send + 'static,
    App: Send + Sync + 'static,
{
    fn call(&self, request: crate::Request<App>, next: Next<App>) -> BoxFuture {
        if request
            .headers()
            .get(header::UPGRADE)
            .is_none_or(|value| value != "websocket")
        {
            return next.call(request);
        }

        if request
            .headers()
            .get(header::SEC_WEBSOCKET_VERSION)
            .is_none_or(|value| value != "13")
        {
            return Box::pin(async {
                raise!(400, message = "sec-websocket-version header must be \"13\"");
            });
        }

        let Some(accept) = request
            .headers()
            .get(header::SEC_WEBSOCKET_KEY)
            .map(|value| gen_accept_key(value.as_ref()))
        else {
            return Box::pin(async {
                raise!(400, message = "missing required header: sec-websocket-key.")
            });
        };

        tokio::spawn({
            let mut request = Request(Arc::new(request));
            let listener = Arc::clone(&self.listen);
            let config = self.config;

            async move {
                if let Some(original) = Arc::get_mut(&mut request.0) {
                    let mut upgradeable = http::Request::new(());

                    mem::swap(original.extensions_mut(), upgradeable.extensions_mut());

                    if let Err(error) = match upgrade(config, &mut upgradeable).await {
                        Err(error) => Err(error),
                        Ok(stream) => {
                            mem::swap(original.extensions_mut(), upgradeable.extensions_mut());
                            start(Box::pin(stream), listener, request).await
                        }
                    } {
                        eprintln!("error(ws): {}", error);
                    }
                }

                if cfg!(debug_assertions) {
                    eprintln!("info(ws): websocket session ended");
                }
            }
        });

        Box::pin(async {
            Response::build()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(header::CONNECTION, "upgrade")
                .header(header::SEC_WEBSOCKET_ACCEPT, accept)
                .header(header::UPGRADE, "websocket")
                .finish()
        })
    }
}
