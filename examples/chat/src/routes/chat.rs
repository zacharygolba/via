use via::ws::{self, Message};
use via::{Next, Payload, Request};

use crate::chat::{Chat, EventParams};

pub async fn join(mut channel: ws::Channel, context: ws::Context<Chat>) -> via::Result<()> {
    let (user, mut updates) = {
        let Some(id) = context
            .cookies()
            .private(context.state().secret())
            .get("via-chat-session")
            .and_then(|cookie| cookie.value().parse().ok())
        else {
            eprintln!("unauthorized");
            return Ok(());
        };

        match context.state().join(&id).await {
            Some(joined) => joined,
            None => {
                eprintln!("unauthorized - invalid user id");
                return Ok(());
            }
        }
    };

    loop {
        tokio::select! {
            // Pubsub
            Ok((ref from_id, event)) = updates.recv() => {
                if from_id != user.id() {
                    channel.send(event).await?;
                }
            }

            // WebSocket
            Some(next) = channel.next() => match next {
                // Append the message content to the chat thread.
                payload @ (Message::Binary(_) | Message::Text(_)) => {
                    let chat = context.state();

                    let params = payload.serde_json_untagged::<EventParams>()?;
                    let event = chat.insert((&user, params)).await?;

                    chat.broadcast(&user, event).await?;
                }

                // Break the loop when we receive a close message.
                Message::Close(close) => {
                    close.as_ref().inspect(|(code, reason)| {
                        let reason = reason.as_deref().unwrap_or("reason not provided");
                        eprintln!("{:?}: {}", code, reason);
                    });

                    break Ok(());
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
