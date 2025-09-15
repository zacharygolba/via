use serde_json::json;
use via::builtin::ws::{WebSocket, WebSocketRequest};
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

pub async fn join(mut socket: WebSocket, request: WebSocketRequest<Chat>) -> Result<(), Error> {
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
            next = socket.next() => match next {
                Ok(Some(message)) => {
                    let text = message.validate_utf8()?;
                    let chat = request.state();

                    if let Some(index) = chat.append(&slug, text).await {
                        let _ = tx.send((id, index));
                    }
                }
                Ok(None) => {
                    break Ok(());
                }
                Err(e) => {
                    eprintln!("error(room: {}): {}", slug, e);
                }
            }
        }
    }
}
