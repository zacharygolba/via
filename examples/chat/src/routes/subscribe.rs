use diesel::SelectableHelper;
use diesel_async::RunQueryDsl;
use via::Payload;
use via::ws::Message::{Binary, Close, Text};
use via::ws::{self, Channel, Request, Retry};

use crate::chat::{Chat, Event};
use crate::models::message::{Message, MessageParams};
use crate::util::Authenticate;

pub async fn subscribe(mut channel: Channel, request: Request<Chat>) -> ws::Result {
    let user = request.current_user().or_break()?.clone();
    let mut rx = request.state().subscribe();

    loop {
        tokio::select! {
            // WebSocket
            Some(next) = channel.recv() => match next {
                // Append the message content to the chat thread.
                payload @ (Binary(_) | Text(_)) => {
                    let trx = async {
                        let mut params = payload.serde_json_untagged::<MessageParams>()?;
                        let state = request.state();

                        params.author_id = Some(user.id);

                        let message = diesel::insert_into(Message::TABLE)
                            .values(params)
                            .returning(Message::as_returning())
                            .get_result(&mut state.pool().get().await?).await?;

                        let event = Event::Message(&message);

                        state.publish(user.id, message.thread_id, event)
                    };

                    trx.await.or_continue()?;
                }

                // Break the loop when we receive a close message.
                Close(close) => {
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
            Ok((ref from_user_id, ref _in_thread_id, event)) = rx.recv() => {
                if &user.id != from_user_id
                    // && user.threads.contains(_in_thread_id)
                {
                    channel.send(event).await?;
                }
            }
        }
    }
}
