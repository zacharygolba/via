use via::ws::{self, Message, Retry};
use via::{Next, Payload, Request, raise};

use crate::chat::{Chat, EventParams};

pub async fn join(mut channel: ws::Channel, request: ws::Request<Chat>) -> ws::Result {
    let (user, mut updates) = {
        let Some(id) = request
            .cookies()
            .private(request.state().secret())
            .get("via-chat-session")
            .and_then(|cookie| cookie.value().parse().ok())
        else {
            return raise!(-> 401).or_break();
        };

        match request.state().join(&id).await {
            Some(joined) => joined,
            None => {
                let message = format!("invalid user id: {}", id);
                return raise!(-> 401, message = message).or_break();
            }
        }
    };

    loop {
        tokio::select! {
            // Pubsub
            Ok((ref from_id, event)) = updates.recv() => {
                if from_id != user.id() {
                    channel.send(event).await.or_continue()?;
                }
            }

            // WebSocket
            Some(next) = channel.next() => match next {
                // Append the message content to the chat thread.
                payload @ (Message::Binary(_) | Message::Text(_)) => {
                    let state = request.state();
                    let trx = async {
                        let params = payload.serde_json_untagged::<EventParams>()?;
                        let event = state.insert((&user, params)).await?;

                        state.broadcast(&user, event).await
                    };

                    trx.await.or_continue()?;
                }

                // Break the loop when we receive a close message.
                Message::Close(close) => {
                    close.as_ref().inspect(|(code, reason)| {
                        let reason = reason.as_deref().unwrap_or("reason not provided");
                        eprintln!("{:?}: {}", code, reason);
                    });

                    return Ok(());
                }

                // Print a warning to stderr for ignored messages.
                ignore => {
                    eprintln!("warn(ignored): {:?}", ignore);
                }
            },
        }
    }
}

pub async fn index(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    todo!()
}

pub async fn message(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    todo!()
}

pub async fn reaction(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    todo!()
}
