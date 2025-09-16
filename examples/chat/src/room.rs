use serde_json::json;
use via::builtin::ws::{Message, RequestContext, WebSocket};
use via::{Error, Next, Request, Response, error};

use crate::chat::Chat;

pub async fn index(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let all = request.state().all(&slug, |messages| {
        let body = json!({ "data": { "messages": messages } });
        Response::build().json(&body)
    });

    if let Some(result) = all.await {
        result
    } else {
        Err(error!(404))
    }
}

pub async fn show(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let index = request.param("index").parse()?;

    if let Some(message) = request.state().get(&slug, index).await {
        let body = json!({ "data": { "message": &message } });
        Response::build().json(&body)
    } else {
        Err(error!(404))
    }
}

pub async fn join(mut socket: WebSocket, request: RequestContext<Chat>) -> Result<(), Error> {
    let slug = request.param("room").into_result()?;
    let (id, (tx, mut rx)) = request.state().join(&slug).await;

    loop {
        tokio::select! {
            Ok((from, index)) = rx.recv() => {
                let chat = request.state();
                if id != from
                    && let Some(message) = chat.get(&slug, index).await
                {
                    socket.send(message).await?;
                }
            }
            result = socket.next() => match result {
                Ok(Message::Binary(_binary)) => {
                    eprintln!("warn(room: {}): binary messages are ignored.", &slug);
                }
                Ok(Message::Close(close)) => {
                    if let Some((code, reason)) = close {
                        eprint!("close(room: {}): {}", &slug, u16::from(code));
                        if let Some(message) = reason.as_deref() {
                            eprintln!(" {}", message);
                        } else {
                            eprintln!("");
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    let chat = request.state();
                    let message = text.into();

                    if let Some(index) = chat.push(&slug, message).await {
                        let _ = tx.send((id, index));
                    }
                }
                Err(error) => {
                    eprintln!("error(room: {}): {}", slug, error);
                }
            }
        }
    }
}
