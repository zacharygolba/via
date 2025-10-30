use bytestring::ByteString;
use serde::Serialize;
use via::websocket::{Channel, Context, Message};
use via::{Next, Payload, Request, Response, raise};

use crate::chat::Chat;

#[derive(Serialize)]
struct ChatThread<'a> {
    messages: &'a [ByteString],
}

#[derive(Serialize)]
struct ChatMessage {
    message: ByteString,
}

pub async fn index(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let all = request.state().all(&slug, |messages| {
        Response::build().json(&ChatThread { messages })
    });

    all.await.unwrap_or_else(|| raise!(404))
}

pub async fn show(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let slug = request.param("room").into_result()?;
    let index = request.param("index").parse()?;
    let Some(message) = request.state().get(&slug, index).await else {
        raise!(404)
    };

    Response::build().json(&ChatMessage { message })
}

pub async fn join(mut channel: Channel, context: Context<Chat>) -> via::Result<()> {
    let slug = context.param("room").into_result()?.into_owned();
    let state = context.into_state();

    let (id, pubsub) = state.join(&slug).await;
    let (tx, mut rx) = pubsub;

    loop {
        tokio::select! {
            // Update received from the room's broadcast channel.
            Ok((from, index)) = rx.recv() => {
                if id == from {
                    continue;
                }

                if let Some(content) = state.get(&slug, index).await {
                    channel.send(content).await?;
                }
            }

            // Message received from the websocket.
            Some(next) = channel.next() => match next {
                // Append the message content to the chat thread.
                content @ (Message::Binary(_) | Message::Text(_)) => {
                    let message = content.into_utf8()?;

                    if let Some(index) = state.push(&slug, message).await {
                        let _ = tx.send((id, index));
                    }
                }

                // Break the loop when we receive a close message.
                Message::Close(close) => {
                    close.as_ref().inspect(|(code, reason)| {
                        let reason = reason.as_deref().unwrap_or("reason not provided");
                        eprintln!("close(room: {}, code: {:?}): {}", slug, code, reason);
                    });

                    break Ok(());
                }

                // Print a warning to stderr for ignored messages.
                ignore => {
                    eprintln!("warn(room: {}): ignored message in room::join {:?}", slug, ignore);
                }
            },
        }
    }
}
