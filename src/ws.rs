use base64::Engine;
use base64::engine::general_purpose::STANDARD as base64_engine;
use futures::{SinkExt, StreamExt};
use http::{StatusCode, header};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Sender};
use tokio_websockets::server::Builder;
use tokio_websockets::{Config, Limits, WebSocketStream};

use crate::Response;
use crate::error::BoxError;
use crate::middleware::{BoxFuture, Middleware};
use crate::next::Next;
use crate::request::Request;

pub use tokio_websockets::{Message, Payload};

const WEB_SOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub type OnMessage<T> =
    dyn Fn(WebSocket<T>, Option<String>) -> BoxFuture<Result<(), BoxError>> + Send + Sync;

pub struct WebSocket<T = ()> {
    state: Arc<T>,
    stream: WebSocketStream<TokioIo<TokioIo<TcpStream>>>,
}

pub struct WsConfig<T> {
    config: Config,
    limits: Limits,
    param_name: Option<String>,
    on_message: Arc<OnMessage<T>>,
}

fn handle_error(error: &(dyn Error + 'static)) {
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

impl<T> WsConfig<T> {
    pub(crate) fn new(param_name: Option<String>, on_message: Arc<OnMessage<T>>) -> Self {
        Self {
            config: Default::default(),
            limits: Default::default(),
            on_message,
            param_name,
        }
    }

    fn extract_param(&self, request: &Request<T>) -> Option<String> {
        let name = self.param_name.as_ref()?;
        let value = request.param(name).into_result().ok()?;

        Some(value.into_owned())
    }
}

impl<T: Send + Sync + 'static> Middleware<T> for WsConfig<T> {
    fn call(&self, request: Request<T>, next: Next<T>) -> crate::BoxFuture {
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

            let param = self.extract_param(&request);
            let (head, _) = request.into_parts();
            let mut request = http::Request::from_parts(head.parts, ());

            let builder = Builder::new().config(self.config).limits(self.limits);
            let state = head.state;
            let f = Arc::clone(&self.on_message);

            tokio::spawn(async move {
                let stream = match hyper::upgrade::on(&mut request)
                    .await
                    .and_then(|upgraded| Ok(upgraded.downcast().unwrap().io))
                {
                    Ok(io) => builder.serve(TokioIo::new(io)),
                    Err(error) => return handle_error(&error),
                };

                if let Err(error) = f(WebSocket { state, stream }, param).await {
                    handle_error(&*error);
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

impl<T> WebSocket<T> {
    #[inline]
    pub fn state(&self) -> &Arc<T> {
        &self.state
    }

    pub async fn next(&mut self) -> Option<Result<Message, BoxError>> {
        Some(self.stream.next().await?.map_err(|e| e.into()))
    }

    pub async fn send(&mut self, message: Message) -> Result<(), BoxError> {
        Ok(self.stream.send(message).await?)
    }
}
