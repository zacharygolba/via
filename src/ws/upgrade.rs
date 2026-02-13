use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use std::mem::{self, swap};
use std::ops::ControlFlow::{Break, Continue};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::coop;
use tungstenite::protocol::{Role, WebSocketConfig};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use super::channel::Channel;
use super::error::{WebSocketError, is_recoverable};
use crate::app::Shared;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::raise;
use crate::request::Envelope;
use crate::response::Response;

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB

type WebSocketStream = tokio_tungstenite::WebSocketStream<TokioIo<Upgraded>>;

#[derive(Debug)]
pub struct Request<App = ()> {
    envelope: Arc<Envelope>,
    app: Shared<App>,
}

pub struct Upgrade<F> {
    listen: Arc<F>,
    config: WebSocketConfig,
}

macro_rules! debug {
    (#[$ctx:meta], $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprint!("error(ws: {}) = ", stringify!($ctx));
            eprintln!($($arg)*);
        }
    };
}

fn gen_accept_key(key: &[u8]) -> String {
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);

    hasher.update(key);
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

    base64_engine.encode(hasher.finish())
}

async fn start<App, F, R>(listen: Arc<F>, config: WebSocketConfig, mut request: crate::Request<App>)
where
    F: Fn(Channel, Request<App>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send,
{
    let mut upgradeable = http::Request::new(());
    swap(request.extensions_mut(), upgradeable.extensions_mut());

    let mut stream = match hyper::upgrade::on(&mut upgradeable).await {
        Err(error) => return debug!(#[upgrade], "{}", error),
        Ok(upgraded) => {
            let io = TokioIo::new(upgraded);

            swap(request.extensions_mut(), upgradeable.extensions_mut());
            WebSocketStream::from_raw_socket(io, Role::Server, Some(config)).await
        }
    };

    let request = {
        let (envelope, _, app) = request.into_parts();
        Request::new(app, envelope)
    };

    'session: loop {
        let (sender, mut rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);
        let mut listener = {
            let channel = Channel::new(sender, receiver);
            Box::pin(listen(channel, request.clone()))
        };

        loop {
            tokio::select! {
                biased;

                // The future returned from app code is ready.
                result = listener.as_mut() => {
                    if let Err(ref flow @ (Break(ref error) | Continue(ref error))) = result {
                        debug!(#[listener], "{}", error);
                        if flow.is_continue() {
                            continue 'session;
                        }
                    }

                    break 'session;
                },

                // Forward the outbound message to the stream.
                Some(next) = coop::unconstrained(rx.recv()) => {
                    coop::consume_budget().await;

                    if let Err(error) = stream.feed(next).await {
                        debug!(#[socket], "{}", error);
                        if !is_recoverable(&error) {
                            break 'session;
                        }
                    }
                }

                // Forward the incoming message to the channel.
                Some(result) = stream.next() => {
                    let error = match result {
                        Err(error) => error,
                        Ok(message) => match tx.send(message).await {
                            Err(_) => WebSocketError::AlreadyClosed,
                            Ok(_) => continue,
                        },
                    };

                    debug!(#[socket], "{}", error);
                    if !is_recoverable(&error) {
                        break 'session;
                    }
                }
            }
        }
    }

    if cfg!(debug_assertions) {
        println!("websocket session ended");
    }
}

impl<App> Request<App> {
    #[inline]
    pub fn app(&self) -> &Shared<App> {
        &self.app
    }

    #[inline]
    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }
}

impl<App> Request<App> {
    fn new(app: Shared<App>, envelope: Envelope) -> Self {
        Self {
            envelope: Arc::new(envelope),
            app,
        }
    }
}

impl<App> Clone for Request<App> {
    fn clone(&self) -> Self {
        Self {
            envelope: self.envelope.clone(),
            app: self.app.clone(),
        }
    }
}

impl<F> Upgrade<F> {
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

impl<T> Clone for Upgrade<T> {
    fn clone(&self) -> Self {
        Self {
            listen: Arc::clone(&self.listen),
            config: self.config,
        }
    }
}

impl<T, Await, App> Middleware<App> for Upgrade<T>
where
    T: Fn(Channel, Request<App>) -> Await + Send + Sync + 'static,
    Await: Future<Output = super::Result> + Send + 'static,
    App: Send + Sync + 'static,
{
    fn call(&self, request: crate::Request<App>, next: Next<App>) -> BoxFuture {
        let headers = request.envelope().headers();

        if headers
            .get(header::UPGRADE)
            .is_none_or(|value| value != "websocket")
        {
            return next.call(request);
        }

        if headers
            .get(header::SEC_WEBSOCKET_VERSION)
            .is_none_or(|value| value != "13")
        {
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
            let Upgrade { ref listen, config } = *self;
            let task = start(Arc::clone(listen), config, request);

            println!("task = {}", mem::size_of_val(&task));

            task
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
