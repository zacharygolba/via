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
    let chat = request.state();

    let (id, (tx, mut rx)) = chat.join(&slug).await;

    loop {
        tokio::select! {
            // Update received from the room's broadcast channel.
            Ok((from, index)) = rx.recv() => {
                if id == from {
                    continue; // Skip updates if they came from us.
                }

                if let Some(message) = chat.get(&slug, index).await {
                    socket.send(message).await?;
                }
            }

            // Message received from the websocket.
            Some(message) = socket.next() => match message {
                // We ignore binary messages in this example but still want to
                // know if we receive one.
                Message::Binary(_binary) => {
                    eprintln!("warn(room: {}): binary messages are ignored.", &slug);
                }

                // Break the loop when we receive a close message.
                Message::Close(close) => {
                    if let Some((code, reason)) = close {
                        let code = u16::from(code);
                        let reason = reason.as_deref().unwrap_or("reason not provided");

                        eprintln!("close(room: {}, code: {}): {}", &slug, code, reason);
                    }

                    break Ok(());
                }

                // Append the message content to the room thread and cast an
                // update.
                Message::Text(text) => {
                    if let Some(index) = chat.push(&slug, &text).await {
                        let _ = tx.send((id, index));
                    }
                }
            },
        }
    }
}
