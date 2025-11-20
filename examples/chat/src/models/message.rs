pub use crate::schema::messages;

use chrono::{DateTime, Utc};
use diesel::associations::HasTable;
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Thread, User};
use crate::models::reaction::ReactionPreview;
use crate::models::user::{UserPreview, users};
use crate::util::Id;

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Thread))]
#[diesel(belongs_to(User, foreign_key = author_id))]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: Id,
    body: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    reactions_count: i64,

    #[serde(skip)]
    author_id: Id,

    #[serde(skip)]
    thread_id: Id,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = messages)]
pub struct ChangeSet {
    body: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = messages)]
#[serde(rename_all = "camelCase")]
pub struct NewMessage {
    pub author_id: Option<Id>,
    pub thread_id: Option<Id>,

    body: String,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(Pg))]
pub struct MessageWithAuthor {
    #[diesel(embed)]
    #[serde(flatten)]
    message: Message,

    #[diesel(embed)]
    author: UserPreview,
}

#[derive(Serialize)]
pub struct MessageWithJoins {
    #[serde(flatten)]
    message: Message,
    author: UserPreview,
    reactions: Vec<ReactionPreview>,
}

filters! {
    pub fn by_id(id == &Id) on messages;
    pub fn by_author(author_id == &Id) on messages;
    pub fn by_thread(thread_id == &Id) on messages;
}

sorts! {
    pub fn recent(#[desc] created_at, id) on messages;
}

pub fn group_by_message(
    messages: Vec<MessageWithAuthor>,
    reactions: Vec<ReactionPreview>,
) -> Vec<MessageWithJoins> {
    let iter = reactions.grouped_by(&messages).into_iter();

    iter.zip(messages)
        .map(|(reactions, message)| message.joins(reactions))
        .collect()
}

impl Message {
    pub fn query() -> messages::table {
        messages::table
    }

    pub fn with_author() -> InnerJoin<messages::table, users::table> {
        Self::query().inner_join(users::table)
    }
}

impl MessageWithAuthor {
    pub fn joins(self, reactions: Vec<ReactionPreview>) -> MessageWithJoins {
        MessageWithJoins {
            message: self.message,
            author: self.author,
            reactions,
        }
    }
}

impl HasTable for MessageWithAuthor {
    type Table = messages::table;

    fn table() -> Self::Table {
        messages::table
    }
}

impl<'a> Identifiable for &'_ &'a MessageWithAuthor {
    type Id = <&'a Message as Identifiable>::Id;

    fn id(self) -> Self::Id {
        Identifiable::id(*self)
    }
}

impl<'a> Identifiable for &'a MessageWithAuthor {
    type Id = <&'a Message as Identifiable>::Id;

    fn id(self) -> Self::Id {
        Identifiable::id(&self.message)
    }
}
