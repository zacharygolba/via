use bytestring::ByteString;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::Payload;
use via::ws::{self, Channel, CloseCode, Message, Request, Retry};

use crate::chat::{Chat, Event, EventContext};
use crate::models::message::{Message as MessageModel, NewMessage};
use crate::util::Authenticate;

fn on_close(close: &(CloseCode, Option<ByteString>)) {
    let (code, reason) = close;
    let reason = reason.as_deref().unwrap_or("reason not provided");

    eprintln!("{:?}: {}", code, reason);
}

pub async fn subscribe(mut channel: Channel, request: Request<Chat>) -> ws::Result {
    let user = request.current_user().or_break()?.clone();
    let mut rx = request.state().subscribe();

    loop {
        tokio::select! {
            // WebSocket
            Some(next) = channel.recv() => match next {
                // Break the loop when we receive a close message.
                Message::Close(close) => {
                    close.inspect(on_close);
                    break Ok(());
                }

                // Append the message content to the chat thread.
                Message::Text(text) => {
                    let mut params = text.serde_json::<NewMessage>().or_continue()?;
                    let thread_id = params.thread_id;

                    params.author_id = Some(user.id);

                    let state = request.state();
                    let result = diesel::insert_into(MessageModel::TABLE)
                        .values(params)
                        .returning(MessageModel::as_returning())
                        .get_result(&mut state.pool().get().await.or_continue()?)
                        .await;

                    let event = Event::Message(result.or_continue()?);
                    let context = EventContext::new(thread_id, user.id);

                    state.publish(context, event).or_continue()?;
                }

                // Print a warning to stderr for ignored messages.
                ignore => {
                    eprintln!("warn(ignored): {:?}", ignore);
                }
            },

            // Pubsub
            Ok((ref context, payload)) = rx.recv() => {
                let _ = context.thread_id();

                if &user.id == context.user_id() {
                    continue;
                }

                channel.send(payload).await?;
            }
        }
    }
}
