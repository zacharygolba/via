use chrono::{DateTime, Utc};
use diesel::dsl::Desc;
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Integer, Text, Uuid};
use serde::{Deserialize, Serialize};

use super::User;
use super::message::{Message, MessageWithAuthor};
use crate::models::Connection;
use crate::models::user::UserPreview;
use crate::schema::{reactions, users};
use crate::util::{DebugQueryDsl, Emoji, Id, sql};

type Pk = reactions::id;
type Table = reactions::table;

type CreatedAtDesc = (Desc<reactions::created_at>, Desc<Pk>);

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
    pub emoji: Emoji,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct NewReaction {
    pub emoji: Emoji,
    pub message_id: Option<Id>,
    pub user_id: Option<Id>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct ReactionIncludes {
    #[diesel(embed)]
    #[serde(flatten)]
    reaction: Reaction,

    #[diesel(embed)]
    user: UserPreview,
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

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    reactions::id.eq(id)
}

pub fn by_message(id: &Id) -> sql::ById<'_, reactions::message_id> {
    reactions::message_id.eq(id)
}

pub fn by_user(id: &Id) -> sql::ById<'_, reactions::user_id> {
    reactions::user_id.eq(id)
}

impl Reaction {
    pub const ID: Pk = reactions::id;

    pub fn created_at_desc() -> CreatedAtDesc {
        (reactions::created_at.desc(), reactions::id.desc())
    }

    pub fn create(values: NewReaction) -> sql::Insert<Table, NewReaction> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        reactions::table
    }

    pub fn includes() -> InnerJoin<Table, users::table> {
        Self::table().inner_join(User::table())
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
