use via::ws::{self, Channel, Message, Retry};
use via::{Next, Payload, Request, Response, raise};

use crate::models::{Chat, EventParams};

pub async fn join(mut channel: Channel, request: ws::Request<Chat>) -> ws::Result {
    let (user_id, mut updates) = {
        let chat = request.state();

        let Some(id) = request
            .cookies()
            .private(chat.secret())
            .get("via-chat-session")
            .and_then(|cookie| cookie.value().parse().ok())
        else {
            return raise!(-> 401).or_break();
        };

        let Some(rx) = chat.subscribe(&id).await else {
            let message = format!("invalid user id: {}", id);
            return raise!(-> 401, message = message).or_break();
        };

        (id, rx)
    };

    loop {
        tokio::select! {
            // Pubsub
            Ok((ref from_id, event)) = updates.recv() => {
                if from_id != &user_id {
                    channel.send(event).await?;
                }
            }

            // WebSocket
            Some(next) = channel.recv() => match next {
                // Append the message content to the chat thread.
                payload @ (Message::Binary(_) | Message::Text(_)) => {
                    let trx = async {
                        let params = payload.serde_json_untagged::<EventParams>()?;
                        request.state().insert_and_notify(&user_id, params).await
                    };

                    trx.await.or_continue()?;
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

pub async fn message(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    todo!()
}

pub async fn reaction(_: Request<Chat>, _: Next<Chat>) -> via::Result {
    todo!()
}

pub async fn thread(request: Request<Chat>, _: Next<Chat>) -> via::Result {
    let id = request.param("id").parse()?;
    let chat = request.state().as_ref();
    let future = chat.thread(&id, |thread| Response::build().json(&thread));

    future.await.unwrap_or_else(|| raise!(404))
}
