use chrono::{DateTime, Utc};
use diesel::associations::HasTable;
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use via::raise;

use super::channel::Channel;
use super::user::{User, UserPreview};
use crate::chat::Connection;
use crate::models::reaction::{Reaction, ReactionPreview};
use crate::schema::{conversations, users};
use crate::util::paginate::PER_PAGE;
use crate::util::{DebugQueryDsl, Id};

#[derive(Associations, Deserialize, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Conversation, foreign_key = thread_id))]
#[diesel(belongs_to(Channel))]
#[diesel(belongs_to(User))]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    id: Id,
    body: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    total_reactions: i64,
    total_replies: i64,

    channel_id: Id,
    thread_id: Option<Id>,
    user_id: Id,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = conversations)]
#[serde(rename_all = "camelCase")]
pub struct NewConversation {
    pub channel_id: Option<Id>,
    pub thread_id: Option<Id>,
    pub user_id: Option<Id>,
    body: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = conversations)]
pub struct ChangeSet {
    body: String,
}

#[derive(Deserialize, Serialize)]
pub struct ConversationDetails {
    #[serde(flatten)]
    conversation: Conversation,

    reactions: Vec<ReactionPreview>,

    user: UserPreview,
}

#[derive(Clone)]
pub struct ConversationParams {
    pub thread_id: Id,
    pub reply_id: Option<Id>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = conversations)]
#[diesel(check_for_backend(Pg))]
pub struct ConversationWithUser {
    #[diesel(embed)]
    #[serde(flatten)]
    conversation: Conversation,

    #[diesel(embed)]
    user: UserPreview,
}

#[derive(Deserialize, Serialize)]
pub struct ThreadDetails {
    #[serde(flatten)]
    conversation: Conversation,

    reactions: Vec<ReactionPreview>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    replies: Vec<ConversationDetails>,

    user: UserPreview,
}

filters! {
    pub fn by_id(id == &Id) on conversations;
    pub fn by_user(user_id == &Id) on conversations;
    pub fn by_thread(thread_id == &Id) on conversations;
    pub fn by_channel(channel_id == &Id) on conversations;

    pub fn is_thread(thread_id is_null) on conversations;
}

sorts! {
    pub fn recent(#[desc] created_at, id) on conversations;
}

fn split_own_reactions(reactions: &mut Vec<ReactionPreview>, id: &Id) -> Vec<ReactionPreview> {
    // Sort the reactions vec so that our own reactions are last.
    reactions.sort_by_key(|reaction| *id == reaction.to_id());

    // Find the first index of a reaction that belongs to the id predicate.
    let pivot = reactions
        .iter()
        .position(|reaction| *id == reaction.to_id())
        .unwrap_or(reactions.len());

    reactions.split_off(pivot)
}

impl Conversation {
    pub async fn find(connection: &mut Connection<'_>, id: &Id) -> via::Result<ThreadDetails> {
        // Load the conversation by id along with the first page of replies.
        let mut conversations = Conversation::with_author()
            .select(ConversationWithUser::as_select())
            .filter(by_id(id).or(by_thread(id)))
            .order(recent())
            .limit(PER_PAGE + 1)
            .debug_load(connection)
            .await?;

        // Aggregate the reactions for each conversation.
        let mut reactions = {
            let ids = conversations.iter().map(Identifiable::id);
            Reaction::to_conversations(connection, ids).await?
        };

        // Take the last conversation from the vec of conversations. The query
        // orders conversations by created_at DESC. Therefore, the last item in the
        // vec is always the parent.
        let Some(conversation) = conversations.pop() else {
            raise!(404, DieselError::NotFound);
        };

        // Move the reactions to conversation into their own vec.
        let own_reactions = split_own_reactions(&mut reactions, id);

        // Group replies to the thread with with their reactions.
        let replies = ConversationDetails::grouped_by(conversations, reactions);

        Ok(conversation.into_thread(own_reactions, replies))
    }

    pub fn query() -> conversations::table {
        conversations::table
    }

    pub fn with_author() -> InnerJoin<conversations::table, users::table> {
        Self::query().inner_join(users::table)
    }
}

impl ConversationDetails {
    pub fn grouped_by(
        conversations: Vec<ConversationWithUser>,
        reactions: Vec<ReactionPreview>,
    ) -> Vec<ConversationDetails> {
        let iter = reactions.grouped_by(&conversations).into_iter();

        iter.zip(conversations)
            .map(|(reactions, message)| message.into_details(reactions))
            .collect()
    }
}

impl ConversationWithUser {
    pub fn into_details(self, reactions: Vec<ReactionPreview>) -> ConversationDetails {
        ConversationDetails {
            conversation: self.conversation,
            reactions,
            user: self.user,
        }
    }

    pub fn into_thread(
        self,
        reactions: Vec<ReactionPreview>,
        replies: Vec<ConversationDetails>,
    ) -> ThreadDetails {
        ThreadDetails {
            conversation: self.conversation,
            reactions,
            replies,
            user: self.user,
        }
    }
}

impl HasTable for ConversationWithUser {
    type Table = conversations::table;

    fn table() -> Self::Table {
        conversations::table
    }
}

impl<'a> Identifiable for &'_ &'a ConversationWithUser {
    type Id = <&'a Conversation as Identifiable>::Id;

    fn id(self) -> Self::Id {
        Identifiable::id(*self)
    }
}

impl<'a> Identifiable for &'a ConversationWithUser {
    type Id = <&'a Conversation as Identifiable>::Id;

    fn id(self) -> Self::Id {
        Identifiable::id(&self.conversation)
    }
}
