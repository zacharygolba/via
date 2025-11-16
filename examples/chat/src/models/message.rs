use chrono::NaiveDateTime;
use diesel::dsl::Desc;
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Thread, User};
use crate::models::user::UserPreview;
use crate::schema::{messages, users};
use crate::util::sql::{self, Id};

type Pk = messages::id;
type Table = messages::table;

type CreatedAtDesc = (Desc<messages::created_at>, Desc<Pk>);

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Thread))]
#[diesel(belongs_to(User, foreign_key = author_id))]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: Id,
    body: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,

    #[serde(skip)]
    author_id: Id,

    #[serde(skip)]
    pub thread_id: Id,
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

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct MessageIncludes {
    #[diesel(embed)]
    #[serde(flatten)]
    message: Message,

    #[diesel(embed)]
    author: UserPreview,
}

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    messages::id.eq(id)
}

pub fn by_author(id: &Id) -> sql::ById<'_, messages::author_id> {
    messages::author_id.eq(id)
}

pub fn by_thread(id: &Id) -> sql::ById<'_, messages::thread_id> {
    messages::thread_id.eq(id)
}

pub fn created_at_desc() -> CreatedAtDesc {
    (messages::created_at.desc(), messages::id.desc())
}

impl Message {
    pub fn as_keyset() -> (messages::created_at, Pk) {
        (messages::created_at, messages::id)
    }

    pub fn create(values: NewMessage) -> sql::Insert<Table, NewMessage> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        messages::table
    }

    pub fn includes() -> InnerJoin<Table, users::table> {
        Self::table().inner_join(User::table())
    }
}
