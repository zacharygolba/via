use diesel::prelude::*;
use via::{Finalize, Payload, Response};

use super::authorization::{Ability, Subscriber};
use crate::chat::{Cache, Event, EventContext};
use crate::database::query::ConversationParams;
use crate::models::conversation::*;
use crate::models::{AuthClaims, Reaction};
use crate::schema::{conversations, users};
use crate::util::error::forbidden;
use crate::util::{DebugQueryDsl, Keyset, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    let channel_id = request.can(AuthClaims::VIEW)?;
    let params = request.envelope().params::<ConversationParams>()?;

    let query = {
        let keyset = request.envelope().query::<Keyset>()?;
        let query = conversations::table
            .inner_join(users::table)
            .select(ConversationWithUser::as_select())
            .filter(by_channel(channel_id).and(keyset.before(recent::columns)))
            .order(recent())
            .limit(keyset.limit);

        if params.reply_id.is_none() {
            query.filter(by_thread(&params.thread_id)).into_boxed()
        } else {
            query.filter(is_thread()).into_boxed()
        }
    };

    let conversations = {
        let mut connection = request.app().database().await?;
        let conversations = query.debug_load(&mut connection).await?;
        let reactions = {
            let ids = conversations.iter().map(|row| row.id());
            Reaction::to_conversations(&mut connection, ids).await?
        };

        ConversationDetails::grouped_by(conversations, reactions)
    };

    Response::build().json(&conversations)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    let channel_id = request.can(AuthClaims::WRITE).copied()?;
    let user_id = request.user().copied()?;

    let thread_id = request
        .envelope()
        .param("thread-id")
        .optional()?
        .map(|id| id.parse())
        .transpose()?;

    // Deserialize a new conversation from the request body.
    let (body, app) = request.into_future();
    let mut new_conversation = body.await?.json::<NewConversation>()?;

    new_conversation.channel_id = Some(channel_id);
    new_conversation.thread_id = thread_id;
    new_conversation.user_id = Some(user_id);

    // Acquire a database connection and create the conversation.
    let conversation = diesel::insert_into(conversations::table)
        .values(new_conversation)
        .returning(Conversation::as_returning())
        .debug_result(&mut app.database().await?)
        .await?;

    let event = Event::Message(conversation);
    let context = EventContext::new(Some(channel_id), user_id);
    let response = Response::build().status(201);

    // Notify subscribers that a conversation has been created and respond.
    app.publish(context, event)?.finalize(response)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    request.can(AuthClaims::WRITE)?;

    let params = request.envelope().params::<ConversationParams>()?;
    let thread = match request.app().get(&params).await? {
        Cache::Hit(conversation) => conversation,
        Cache::Miss(key) => {
            let mut connection = request.app().database().await?;
            let conversation = Conversation::find(&mut connection, params.id()).await?;

            request.app().set(&key, &conversation).await?;
            conversation
        }
    };

    Response::build().json(&thread)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let user_id = *request.user()?;
    let params = request.envelope().params::<ConversationParams>()?;

    // Deserialize the request body into conversation params.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and execute the update.
    let Some(conversation) = diesel::update(conversations::table)
        .filter(by_id(params.id()).and(by_user(&user_id)))
        .set(changes)
        .returning(Conversation::as_returning())
        .debug_result(&mut app.database().await?)
        .await
        .optional()?
    else {
        return forbidden();
    };

    Response::build().json(&conversation)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let params = request.envelope().params::<ConversationParams>()?;

    // Acquire a database connection.
    let mut connection = request.app().database().await?;

    if let Err(user_id) = request.subscription()?.can(AuthClaims::MODERATE) {
        // The user that made the request is not a moderator.
        // 403 unless they are the author of the conversation.
        if let ..1 = diesel::delete(conversations::table)
            .filter(by_id(params.id()).and(by_user(&user_id)))
            .debug_execute(&mut connection)
            .await?
        {
            return forbidden();
        }
    } else {
        // The user that made the request is a moderator.
        diesel::delete(conversations::table)
            .filter(by_id(params.id()))
            .debug_execute(&mut connection)
            .await?;
    }

    Response::build().status(204).finish()
}
