use chrono::{DateTime, Utc};
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Integer, Text, Uuid};
use serde::{Deserialize, Serialize};

use super::conversation::ConversationWithUser;
use super::user::{User, UserPreview, users};
use crate::chat::Connection;
use crate::schema::reactions;
use crate::util::{DebugQueryDsl, Emoji, Id};

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(ConversationWithUser, foreign_key = conversation_id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Id,
    emoji: Emoji,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    #[serde(skip)]
    conversation_id: Id,

    #[serde(skip)]
    user_id: Id,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = reactions)]
pub struct ChangeSet {
    emoji: Emoji,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct NewReaction {
    pub conversation_id: Option<Id>,
    pub user_id: Option<Id>,
    emoji: Emoji,
}

#[derive(Associations, Clone, QueryableByName, Serialize)]
#[diesel(belongs_to(ConversationWithUser, foreign_key = conversation_id))]
#[diesel(table_name = reactions)]
#[diesel(check_for_backend(Pg))]
#[serde(rename_all = "camelCase")]
pub struct ReactionPreview {
    emoji: Emoji,

    #[diesel(sql_type = Array<Text>)]
    usernames: Vec<String>,

    #[diesel(sql_type = BigInt)]
    total_count: i64,

    #[serde(skip)]
    conversation_id: Id,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct ReactionWithUser {
    #[diesel(embed)]
    #[serde(flatten)]
    reaction: Reaction,

    #[diesel(embed)]
    user: UserPreview,
}

filters! {
    pub fn by_id(id == &Id) on reactions;
    pub fn by_user(user_id == &Id) on reactions;
}

sorts! {
    pub fn recent(#[desc] created_at, id) on reactions;
}

impl Reaction {
    pub fn query() -> reactions::table {
        reactions::table
    }

    pub fn with_user() -> InnerJoin<reactions::table, users::table> {
        Self::query().inner_join(User::query())
    }

    pub fn to_conversations<'a>(
        connection: &mut Connection<'_>,
        ids: impl IntoIterator<Item = &'a Id>,
    ) -> impl Future<Output = QueryResult<Vec<ReactionPreview>>> {
        const UNIQUE_REACTIONS_PER_CONVERSATION: i32 = 12;
        const USERNAMES_PER_REACTION: i32 = 6;

        diesel::sql_query("SELECT * FROM top_reactions_for($1, $2, $3)")
            .bind::<Array<Uuid>, Vec<_>>(ids.into_iter().collect())
            .bind::<Integer, _>(UNIQUE_REACTIONS_PER_CONVERSATION)
            .bind::<Integer, _>(USERNAMES_PER_REACTION)
            .debug_load(connection)
    }
}

impl ReactionPreview {
    pub fn to_id(&self) -> Id {
        self.conversation_id
    }
}
