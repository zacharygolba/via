use chrono::NaiveDateTime;
use diesel::deserialize::FromSqlRow;
use diesel::dsl::{AsSelect, Desc, Eq, Lt};
use diesel::helper_types::{InnerJoin, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::row::Row;
use serde::{Deserialize, Serialize};

use super::thread::Thread;
use super::user::User;
use crate::schema::messages::{self, dsl as col};
use crate::schema::users;
use crate::util::{Cursor, Id};

pub type TableWithJoins = InnerJoin<messages::table, users::table>;
pub type DefaultSelection = (AsSelect<Message, Pg>, AsSelect<User, Pg>);

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Thread))]
#[diesel(belongs_to(User, foreign_key = author_id))]
#[diesel(table_name = messages)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: Id,

    body: String,

    #[serde(skip)]
    author_id: Id,
    #[serde(skip)]
    pub thread_id: Id,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = messages)]
pub struct ChangeSet {
    pub body: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = messages)]
#[serde(rename_all = "camelCase")]
pub struct NewMessage {
    pub body: String,

    pub author_id: Option<Id>,
    pub thread_id: Option<Id>,
}

#[derive(Serialize)]
pub struct MessageWithAuthor {
    #[serde(flatten)]
    message: Message,
    author: User,
}

pub fn by_author(id: Id) -> Eq<col::author_id, Id> {
    col::author_id.eq(id)
}

pub fn by_cursor(cursor: Cursor) -> Lt<col::created_at, NaiveDateTime> {
    col::created_at.lt(cursor.before)
}

pub fn by_id(id: Id) -> Eq<col::id, Id> {
    col::id.eq(id)
}

pub fn by_thread(thread_id: Id) -> Eq<col::thread_id, Id> {
    col::thread_id.eq(thread_id)
}

pub fn created_at_desc() -> (Desc<col::created_at>, Desc<col::id>) {
    (col::created_at.desc(), col::id.desc())
}

impl Message {
    pub const TABLE: messages::table = messages::table;

    pub fn query() -> Select<TableWithJoins, DefaultSelection> {
        Self::TABLE
            .inner_join(users::table)
            .select((Self::as_select(), User::as_select()))
    }
}

impl FromSqlRow<DefaultSelection, Pg> for MessageWithAuthor {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (message, author) = <_ as FromSqlRow<DefaultSelection, _>>::build_from_row(row)?;
        Ok(MessageWithAuthor { message, author })
    }
}
