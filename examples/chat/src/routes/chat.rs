use diesel::prelude::*;
use std::collections::HashMap;
use via::request::Payload;
use via::ws::{self, Channel, Message, Request, Retry};

use crate::chat::{Chat, Event, EventContext};
use crate::models::message::NewMessage;
use crate::models::subscription::{AuthClaims, Subscription};
use crate::schema::subscriptions;
use crate::util::{DebugQueryDsl, Id, Session};

/// Prints the format string to stderr in debug builds.
///
macro_rules! debug {
    ($($args:tt)+) => { if cfg!(debug_assertions) { eprintln!($($args)+); } };
}

pub async fn chat(mut channel: Channel, request: Request<Chat>) -> ws::Result {
    // The current user that opened the websocket.
    let user = request.current_user().cloned().or_break()?;

    // Subscribe to event notifications from peers.
    let mut pubsub = request.state().subscribe();

    // The current users thread subscription claims keyed by thread id.
    let subscriptions: HashMap<Id, AuthClaims> = {
        let acquire = request.state().pool().get().await;
        let result = Subscription::belonging_to(&user)
            .select((subscriptions::thread_id, subscriptions::claims))
            .debug_load::<(Id, AuthClaims)>(&mut acquire.or_break()?)
            .await;

        result.or_break()?.into_iter().collect()
    };

    'recv: loop {
        let new_message = 'send: {
            tokio::select! {
                // Received a message from the websocket channel.
                Some(message) = channel.recv() => match message {
                    Message::Text(mut payload) => {
                        let mut new_message = payload.json::<NewMessage>().or_continue()?;

                        // Confirm that the current user can write in the
                        // thread before we proceed.
                        if let Some(id) = &new_message.thread_id
                            && let Some(claims) = subscriptions.get(id)
                            && claims.contains(AuthClaims::WRITE)
                        {
                            new_message.author_id = Some(user.id);
                            break 'send new_message;
                        }
                    }
                    Message::Close(close) => {
                        close.inspect(|context| debug!("{:?}", context));
                        break 'recv Ok(());
                    }
                    Message::Binary(_) => {
                        debug!("warn(chat): ignoring binary message");
                    }
                    ignored => {
                        debug!("warn(chat): ignoring {:?}", ignored);
                    }
                },

                // Received an event notification from another async task.
                Ok((ref context, message)) = pubsub.recv() => {
                    if user.id != context.user_id()
                        && let Some(id) = context.thread_id()
                        && let Some(claims) = subscriptions.get(id)
                        && claims.contains(AuthClaims::VIEW)
                    {
                        channel.send(message).await?;
                    }
                }
            }

            continue 'recv;
        };

        // Build the event context from the request and params. We use this to
        // determine if a message is for the current user in the second arm of
        // the select expression above.
        let context = EventContext::new(new_message.thread_id, user.id);

        // Insert the message into the database and return a message event.
        let event = {
            // Import the message model as late as possible to prevent
            // confusion with the via::ws::Message enum.
            use crate::models::Message;

            // Acquire a database connection and create the message.
            let acquire = request.state().pool().get().await;
            let create = Message::create(new_message)
                .returning(Message::as_returning())
                .debug_result(&mut acquire.or_continue()?)
                .await;

            Event::Message(create.or_continue()?)
        };

        // Publish the event over the broadcast channel to notify peers.
        request.state().publish(context, event).or_continue()?;
    }
}
