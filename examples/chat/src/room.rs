use serde_json::json;
use via::ws::{self, Message};
use via::{Next, Payload, Request, Response};

use crate::chat::Chat;

pub async fn index(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let all = request.state().all(&slug, |messages| {
        let body = json!({ "data": { "messages": messages } });
        Response::build().json(&body)
    });

    all.await.unwrap_or_else(|| Err(via::raise!(404)))
}

pub async fn show(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let index = request.param("index").parse()?;

    if let Some(message) = request.state().get(&slug, index).await {
        let body = json!({ "data": { "message": &message } });
        Response::build().json(&body)
    } else {
        Err(via::raise!(404))
    }
}

pub async fn join(mut channel: ws::Channel, request: ws::Context<Chat>) -> via::Result<()> {
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
                    channel.send(message).await?;
                }
            }

            // Message received from the websocket.
            Some(message) = channel.next() => {
                // Break the loop when we receive a close message.
                if let Message::Close(close) = &message {
                    if let Some((code, reason)) = close {
                        let code = u16::from(*code);
                        let reason = reason.as_deref().unwrap_or("reason not provided");

                        eprintln!("close(room: {}, code: {}): {}", &slug, code, reason);
                    }

                    break Ok(());
                }

                if let Some(index) = chat.push(&slug, message.into_utf8()?).await {
                    let _ = tx.send((id, index));
                }
            },
        }
    }
}
