use via::ws::{self, Channel, Message, Request, Retry};

use crate::chat::Chat;
use crate::util::Authenticate;

pub async fn subscribe(mut channel: Channel, request: Request<Chat>) -> ws::Result {
    let user = request.current_user().or_break()?.clone();
    let mut rx = request.state().subscribe();

    loop {
        tokio::select! {
            // WebSocket
            Some(next) = channel.recv() => match next {
                // Append the message content to the chat thread.
                _payload @ (Message::Binary(_) | Message::Text(_)) => {
                    // let trx = async {
                    //     let params = payload.serde_json_untagged::<EventParams>()?;
                    //     request.state().insert_and_notify(&user_id, params).await
                    // };

                    // trx.await.or_continue()?;
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

            // Pubsub
            Ok((ref from_id, event)) = rx.recv() => {
                if from_id != &user.id {
                    channel.send(event).await?;
                }
            }
        }
    }
}
