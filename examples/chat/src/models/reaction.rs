pub use crate::schema::reactions;

use chrono::{DateTime, Utc};
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Integer, Text, Uuid};
use serde::{Deserialize, Serialize};

use super::message::{Message, MessageWithAuthor};
use crate::models::Connection;
use crate::models::user::{User, UserPreview, users};
use crate::util::{DebugQueryDsl, Emoji, Id};

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(MessageWithAuthor, foreign_key = message_id))]
#[diesel(belongs_to(Message))]
#[diesel(belongs_to(User))]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Id,
    emoji: Emoji,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    #[serde(skip)]
    message_id: Id,

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
    pub message_id: Option<Id>,
    pub user_id: Option<Id>,
    emoji: Emoji,
}

#[derive(Associations, QueryableByName, Serialize)]
#[diesel(belongs_to(MessageWithAuthor, foreign_key = message_id))]
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
    message_id: Id,
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
    pub fn reactions::by_id(id == &Id);
    pub fn reactions::by_user(id == &Id);
    pub fn reactions::by_message(id == &Id);
}

sorts! {
    pub fn reactions::recent(#[desc] created_at, id);
}

impl Reaction {
    pub fn query() -> reactions::table {
        reactions::table
    }

    pub fn with_user() -> InnerJoin<reactions::table, users::table> {
        Self::query().inner_join(User::table())
    }

    pub fn to_messages(
        connection: &mut Connection<'_>,
        ids: Vec<&Id>,
    ) -> impl Future<Output = QueryResult<Vec<ReactionPreview>>> {
        const UNIQUE_REACTIONS_PER_MESSAGE: i32 = 12;
        const USERNAMES_PER_REACTION: i32 = 6;

        diesel::sql_query("SELECT * FROM top_reactions_for($1, $2, $3)")
            .bind::<Array<Uuid>, _>(ids)
            .bind::<Integer, _>(UNIQUE_REACTIONS_PER_MESSAGE)
            .bind::<Integer, _>(USERNAMES_PER_REACTION)
            .debug_load(connection)
    }
}
