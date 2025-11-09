use bytestring::ByteString;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::Payload;
use via::ws::{self, Channel, CloseCode, Message, Request, Retry};

use crate::chat::{Chat, Event, EventContext};
use crate::models::message::NewMessage;
use crate::util::Authenticate;

pub async fn subscribe(mut channel: Channel, request: Request<Chat>) -> ws::Result {
    // The current user that opened the websocket.
    let user = request.head().current_user().or_break()?.clone();

    // Subscribe to event notifications from peers.
    let mut rx = request.state().subscribe();

    loop {
        let mut params: NewMessage = tokio::select! {
            // Received a message from the websocket channel.
            Some(message) = channel.recv() => match message {
                Message::Text(payload) => payload.serde_json().or_continue()?,
                Message::Close(close) => break on_close(close),
                ignored => {
                    eprintln!("warn(/api/subscribe): ignoring {:?}", ignored);
                    continue;
                }
            },

            // Received an event notification from another async task.
            Ok((ref context, message)) = rx.recv() => {
                let _ = context.thread_id();

                // If the event is relevant and not redundant for the current
                // user, send the payload to them with the websocket channel.
                if &user.id != context.user_id() {
                    channel.send(message).await?;
                }

                continue;
            }
        };

        // Build the event context from the request and params. We use this to
        // determine if a message is for the current user in the second arm of
        // the select expression above.
        let context = EventContext::new(params.thread_id, user.id);

        // Insert the message into the database and return a message event.
        let event = {
            // Import the message model as late as possible to prevent
            // confusion with the via::ws::Message enum.
            use crate::models::message::Message;

            // Set the author_id of the message to the current user's id.
            params.author_id = Some(user.id);

            // Acquire a database connection.
            let mut conn = request.state().pool().get().await.or_continue()?;

            // Perform the insert.
            let result = diesel::insert_into(Message::TABLE)
                .values(params)
                .returning(Message::as_returning())
                .get_result(&mut conn)
                .await;

            Event::Message(result.or_continue()?)
        };

        // Publish the event over the broadcast channel to notify peers.
        request.state().publish(context, event).or_continue()?;
    }
}

fn on_close(close: Option<(CloseCode, Option<ByteString>)>) -> ws::Result {
    if let Some((code, reason)) = &close {
        let reason = reason.as_deref().unwrap_or("reason not provided");
        eprintln!("{:?}: {}", code, reason);
    }

    Ok(())
}
