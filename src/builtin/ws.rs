use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures::{SinkExt, StreamExt};
use http::request::Parts;
use http::{StatusCode, header};
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Error as WsError, Limits};

use crate::error::Error;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::param::ParamOffsets;
use crate::request::{Params, PathParam, Request};
use crate::response::Response;

pub use tokio_websockets::{Message, Payload};

const WEB_SOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub type OnUpgrade<T> = dyn Fn(WebSocket, Context<T>) -> BoxFuture<Result<(), Error>> + Send + Sync;

pub struct Context<T> {
    params: Arc<Params>,
    state: Arc<T>,
}

pub struct WebSocket {
    sender: Sender<Message>,
    receiver: Receiver<Result<Message, WsError>>,
}

pub struct WebSocketUpgrade<T> {
    max_payload_len: Option<usize>,
    flush_threshold: usize,
    frame_size: usize,
    on_upgrade: Arc<OnUpgrade<T>>,
}

pub fn ws<T, F, R>(upgraded: F) -> WebSocketUpgrade<T>
where
    F: Fn(WebSocket, Context<T>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send + Sync + 'static,
{
    let frame_size = 4 * 1024;
    let flush_threshold = frame_size * 2;

    WebSocketUpgrade {
        flush_threshold,
        frame_size,
        max_payload_len: None,
        on_upgrade: Arc::new(move |socket, message| Box::pin(upgraded(socket, message))),
    }
}

fn handle_error(error: &(dyn std::error::Error + 'static)) {
    if cfg!(debug_assertions) {
        eprintln!("error(ws): {}", error);
    }
}

fn validate_accept_key<T>(request: &Request<T>) -> Result<String, crate::Error> {
    let name = header::SEC_WEBSOCKET_KEY;

    if let Some(key) = request.header(&name)? {
        let mut hasher = Sha1::new();

        hasher.update(key.as_bytes());
        hasher.update(WEB_SOCKET_GUID.as_bytes());

        Ok(base64_engine.encode(hasher.finalize()))
    } else {
        Err(crate::error!(400, "Required header {} is missing.", name))
    }
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

impl<T: Send + Sync + 'static> Middleware<T> for WebSocketUpgrade<T> {
    fn call(&self, request: Request<T>, next: Next<T>) -> crate::BoxFuture {
        println!("UPGRADE");

        if request
            .header(header::UPGRADE)
            .is_ok_and(|upgrade| upgrade.is_some_and(|value| value == "websocket"))
        {
            if let Err(error) = validate_websocket_version(&request) {
                return Box::pin(async { Err(error) });
            }

            let accept_key = match validate_accept_key(&request) {
                Err(error) => return Box::pin(async { Err(error) }),
                Ok(key) => key,
            };

            let (head, _) = request.into_parts();
            let context = Context::new(&head.parts, head.params, head.state);

            let mut can_upgrade = http::Request::from_parts(head.parts, ());
            let on_upgrade = Arc::clone(&self.on_upgrade);
            let builder = Builder::new()
                .limits(Limits::default().max_payload_len(self.max_payload_len))
                .config(
                    Config::default()
                        .flush_threshold(self.flush_threshold)
                        .frame_size(self.frame_size),
                );

            tokio::spawn(async move {
                let mut stream = match hyper::upgrade::on(&mut can_upgrade).await {
                    Ok(io) => builder.serve(TokioIo::new(io)),
                    Err(error) => return handle_error(&error),
                };

                'session: loop {
                    let (sender, mut rx) = mpsc::channel(128);
                    let (tx, receiver) = mpsc::channel(128);
                    let mut future = on_upgrade(WebSocket { sender, receiver }, context.clone());

                    loop {
                        tokio::select! {
                            Some(next) = stream.next() => drop(match next {
                                Err(WsError::AlreadyClosed) => break 'session,
                                result @ Err(_) => tx.send(result).await,
                                Ok(message) => tx.send(Ok(message)).await,
                            }),
                            Some(message) = rx.recv() => match stream.send(message).await {
                                Err(WsError::AlreadyClosed) => break 'session,
                                Err(error) => handle_error(&error),
                                Ok(_) => {},
                            },
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
            });

            return Box::pin(async {
                Response::build()
                    .status(StatusCode::SWITCHING_PROTOCOLS)
                    .header(header::CONNECTION, "Upgrade")
                    .header(header::UPGRADE, "websocket")
                    .header(header::SEC_WEBSOCKET_ACCEPT, accept_key)
                    .finish()
            });
        }

        next.call(request)
    }
}

impl<T> Context<T> {
    fn new(parts: &Parts, params: ParamOffsets, state: Arc<T>) -> Self {
        Self {
            params: Arc::new(Params::new(parts.uri.path_and_query().cloned(), params)),
            state,
        }
    }
}

impl<T> Context<T> {
    #[inline]
    pub fn path(&self) -> &str {
        self.params.path()
    }

    #[inline]
    pub fn param<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.params.get(name)
    }

    #[inline]
    pub fn state(&self) -> &Arc<T> {
        &self.state
    }
}

impl<T> Clone for Context<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            params: Arc::clone(&self.params),
            state: Arc::clone(&self.state),
        }
    }
}

impl WebSocket {
    pub async fn next(&mut self) -> Option<Result<Message, Error>> {
        match self.receiver.recv().await? {
            Err(error) => Some(Err(error.into())),
            Ok(message) => {
                if message.is_close() {
                    self.receiver.close();
                    None
                } else {
                    Some(Ok(message))
                }
            }
        }
    }

    pub async fn send(&mut self, message: Message) -> Result<(), Error> {
        if let Err(_) = self.sender.send(message).await {
            Err(WsError::AlreadyClosed.into())
        } else {
            Ok(())
        }
    }
}
