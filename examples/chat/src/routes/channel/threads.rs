use diesel::prelude::*;
use via::request::PathParams;
use via::{Error, Finalize, Payload, Response, raise};

use super::authorization::{Ability, Subscriber};
use crate::chat::{Event, EventContext};
use crate::models::conversation::*;
use crate::models::{AuthClaims, Reaction};
use crate::schema::{conversations, users};
use crate::util::error::forbidden;
use crate::util::paginate::PER_PAGE;
use crate::util::{DebugQueryDsl, Id, Keyset, Session};
use crate::{Next, Request};

#[derive(Clone)]
pub(super) struct ThreadArgs {
    thread_id: Option<Id>,
    reply_id: Option<Id>,
}

pub async fn index(request: Request, _: Next) -> via::Result {
    // The current user is subscribed to the channel.
    let channel_id = request.can(AuthClaims::VIEW)?;
    let params = request.envelope().params::<ThreadArgs>()?;

    let query = {
        let keyset = request.envelope().query::<Keyset>()?;
        let query = conversations::table
            .inner_join(users::table)
            .select(ConversationWithUser::as_select())
            .filter(by_channel(channel_id).and(keyset.before(recent::columns)))
            .order(recent())
            .limit(keyset.limit);

        if let Some(id) = &params.thread_id {
            query.filter(by_thread(id)).into_boxed()
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
    let path_args = request.envelope().params::<ThreadArgs>()?;
    let user_id = request.user().copied()?;

    // Deserialize a new conversation from the request body.
    let (body, app) = request.into_future();
    let mut new_conversation = body.await?.json::<NewConversation>()?;

    new_conversation.channel_id = Some(channel_id);
    new_conversation.thread_id = path_args.thread_id;
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
    let channel_id = request.can(AuthClaims::WRITE)?;
    let params = request.envelope().params::<ThreadArgs>()?;
    let id = params.id()?;

    let mut connection = request.app().database().await?;

    // Acquire a database connection and execute the query.
    let conversation = Conversation::with_author()
        .select(ConversationWithUser::as_select())
        .filter(by_id(id).and(by_channel(channel_id)))
        .debug_first(&mut connection)
        .await?;

    let thread = if params.reply_id.is_none() {
        let replies = Conversation::with_author()
            .select(ConversationWithUser::as_select())
            .filter(by_thread(id))
            .order(recent())
            .limit(PER_PAGE)
            .debug_load(&mut connection)
            .await?;

        // The reactions to the thread and the first page of replies.
        let reactions = {
            let ids = replies.iter().map(|reply| reply.id()).chain(Some(id));
            Reaction::to_conversations(&mut connection, ids).await?
        };

        // The reactions to the thread.
        let thread_reactions = reactions
            .iter()
            .filter(|reaction| *id == reaction.to_id())
            .cloned()
            .collect();

        // Replies with their reactions to the thread.
        let replies = ConversationDetails::grouped_by(replies, reactions);

        conversation.into_thread(thread_reactions, Some(replies))
    } else {
        conversation.into_thread(
            Reaction::to_conversations(&mut connection, Some(id)).await?,
            None,
        )
    };

    Response::build().json(&thread)
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let user_id = *request.user()?;
    let id = request.envelope().params::<ThreadArgs>()?.try_into()?;

    // Deserialize the request body into conversation params.
    let (body, app) = request.into_future();
    let changes = body.await?.json::<ChangeSet>()?;

    // Acquire a database connection and execute the update.
    let Some(conversation) = diesel::update(conversations::table)
        .filter(by_id(&id).and(by_user(&user_id)))
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
    let id = request.envelope().params::<ThreadArgs>()?.try_into()?;

    // Acquire a database connection.
    let mut connection = request.app().database().await?;

    if let Err(user_id) = request.subscription()?.can(AuthClaims::MODERATE) {
        // The user that made the request is not a moderator.
        // 403 unless they are the author of the conversation.
        if let ..1 = diesel::delete(conversations::table)
            .filter(by_id(&id).and(by_user(&user_id)))
            .debug_execute(&mut connection)
            .await?
        {
            return forbidden();
        }
    } else {
        // The user that made the request is a moderator.
        diesel::delete(conversations::table)
            .filter(by_id(&id))
            .debug_execute(&mut connection)
            .await?;
    }

    Response::build().status(204).finish()
}

impl ThreadArgs {
    pub fn id(&self) -> via::Result<&Id> {
        let Some(id) = self.reply_id.as_ref().or(self.thread_id.as_ref()) else {
            raise!(400, message = "missing required path parameter: thread-id");
        };

        Ok(id)
    }
}

impl TryFrom<ThreadArgs> for Id {
    type Error = Error;

    fn try_from(params: ThreadArgs) -> Result<Self, Self::Error> {
        params.id().cloned()
    }
}

impl TryFrom<PathParams<'_>> for ThreadArgs {
    type Error = Error;

    fn try_from(params: PathParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            thread_id: params
                .get("thread-id")
                .optional()?
                .map(|id| id.parse())
                .transpose()?,

            reply_id: params
                .get("reply-id")
                .optional()?
                .map(|id| id.parse())
                .transpose()?,
        })
    }
}
