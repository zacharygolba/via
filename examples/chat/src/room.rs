use serde_json::json;
use via::builtin::ws::{Context, Message, WebSocket};
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

pub async fn join(mut socket: WebSocket, context: Context<Chat>) -> Result<(), Error> {
    let slug = context.param("room").into_result()?;
    let chat = context.state();

    let (id, channel) = chat.join(&slug).await;
    let (tx, mut rx) = channel;

    loop {
        tokio::select! {
            Ok((from, index)) = rx.recv() => {
                if id == from {
                    continue;
                }

                if let Some(message) = chat.get(&slug, index).await {
                    socket.send(Message::text(message)).await?;
                }
            }
            next = socket.next() => match next {
                Some(Ok(message)) => {
                    let text = match message.as_text() {
                        Some(utf8) => utf8.to_owned(),
                        None => continue,
                    };

                    if let Some(index) = chat.append(&slug, text).await {
                        let _ = tx.send((id, index));
                    }
                }
                Some(Err(error)) => {
                    eprintln!("error(room: {}): {}", slug, error);
                }
                None => {
                    break Ok(());
                }
            }
        }
    }
}
