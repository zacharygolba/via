use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures_util::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use std::mem::swap;
use std::ops::ControlFlow::{Break, Continue};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::coop;
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits};

#[cfg(feature = "aws-lc-rs")]
use aws_lc_rs::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

#[cfg(feature = "ring")]
use ring::digest::{Context as Hasher, SHA1_FOR_LEGACY_USE_ONLY};

use super::error::{ErrorKind, try_rescue_ws};
use super::message::{Channel, Message};
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::raise;
use crate::request::RequestHead;
use crate::response::Response;

const DEFAULT_FRAME_SIZE: usize = 16 * 1024; // 16KB

#[derive(Debug)]
pub struct Request<State = ()> {
    head: Arc<RequestHead<State>>,
}

pub struct Upgrade<F> {
    config: Config,
    limits: Limits,
    listen: Arc<F>,
}

fn gen_accept_key(key: &[u8]) -> String {
    let mut hasher = Hasher::new(&SHA1_FOR_LEGACY_USE_ONLY);

    hasher.update(key);
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");

    base64_engine.encode(hasher.finish())
}

fn handle_error(error: &impl std::error::Error) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

async fn start<State, F, R>(listen: Arc<F>, mut head: Arc<RequestHead<State>>, builder: Builder)
where
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send,
{
    let stream = {
        let Some(head_mut) = Arc::get_mut(&mut head) else {
            panic!("via::ws::upgrade: handshake already performed.");
        };

        let mut request = http::Request::new(());
        swap(head_mut.extensions_mut(), request.extensions_mut());

        let result = hyper::upgrade::on(&mut request).await;
        swap(request.extensions_mut(), head_mut.extensions_mut());

        match result {
            Ok(upgraded) => builder.serve(TokioIo::new(upgraded)),
            Err(error) => return handle_error(&error),
        }
    };

    tokio::pin!(stream);

    'session: loop {
        let (sender, mut rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);
        let mut listener = Box::pin(listen(
            Channel::new(sender, receiver),
            Request::new(Arc::clone(&head)),
        ));

        loop {
            let flow = tokio::select! {
                biased;

                // The future returned from app code is ready.
                result = listener.as_mut() => match result {
                    Err(Continue(error)) => Continue(Some(error.into())),
                    Err(Break(error)) => Break(Some(error.into())),
                    Ok(_) => Break(None),
                },

                // Forward the outbound message to the stream.
                Some(message) = coop::unconstrained(rx.recv()) => {
                    let result = stream.feed(message.into()).await;

                    coop::consume_budget().await;

                    if let Err(error) = result {
                        try_rescue_ws(error)
                    } else {
                        Continue(None)
                    }
                }

                // Forward the incoming message to the channel.
                Some(result) = stream.next() => {
                    coop::consume_budget().await;

                    match result.and_then(Message::try_from) {
                        Ok(message) => {
                            if tx.send(message).await.is_ok() {
                                Continue(None)
                            } else {
                                Break(Some(ErrorKind::CLOSED))
                            }
                        }
                        Err(error) => try_rescue_ws(error),
                    }
                }
            };

            match &flow {
                Continue(None) => {}
                Continue(Some(error)) => {
                    handle_error(error);
                    if matches!(error, ErrorKind::Listener(_)) {
                        continue 'session;
                    }
                }

                Break(None) => {
                    let _ = stream.flush().await.inspect_err(handle_error);
                    break 'session;
                }
                Break(Some(error)) => {
                    handle_error(error);
                    break 'session;
                }
            }
        }
    }

    if cfg!(debug_assertions) {
        println!("websocket session ended");
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

impl<F> Upgrade<F> {
    pub(super) fn new(upgraded: F) -> Self {
        let frame_size = DEFAULT_FRAME_SIZE;

        Self {
            config: Config::default()
                .flush_threshold(frame_size)
                .frame_size(frame_size),
            limits: Limits::default().max_payload_len(Some(frame_size)),
            listen: Arc::new(upgraded),
        }
    }

    /// The threshold at which the bytes queued at socket are flushed.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn flush_threshold(self, flush_threshold: usize) -> Self {
        Self {
            config: self.config.flush_threshold(flush_threshold),
            ..self
        }
    }

    /// The frame size used for messages in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn frame_size(self, frame_size: usize) -> Self {
        Self {
            config: self.config.frame_size(frame_size),
            ..self
        }
    }

    /// The maximum payload size in bytes.
    ///
    /// **Default:** `16 KB`
    ///
    pub fn max_payload_size(self, max_payload_size: Option<usize>) -> Self {
        Self {
            limits: self.limits.max_payload_len(max_payload_size),
            ..self
        }
    }
}

impl<State, F, R> Middleware<State> for Upgrade<F>
where
    State: Send + Sync + 'static,
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = super::Result> + Send + 'static,
{
    fn call(&self, request: crate::Request<State>, next: Next<State>) -> BoxFuture {
        let headers = request.head().headers();

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
            let (head, _) = request.into_parts();
            let builder = Builder::new().config(self.config).limits(self.limits);
            let listen = Arc::clone(&self.listen);
            let head = Arc::new(head);

            start(listen, head, builder)
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
