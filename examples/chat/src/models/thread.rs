use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::message::MessageIncludes;
use crate::models::subscription::UserSubscription;
use crate::schema::threads;
use crate::util::sql::{self, Id};

type Pk = threads::id;
type Table = threads::table;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Id,
    name: String,
    created_at: NaiveDateTime,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
pub struct NewThread {
    name: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = threads)]
pub struct ChangeSet {
    name: String,
}

#[derive(Serialize)]
pub struct ThreadIncludes {
    #[serde(flatten)]
    pub thread: Thread,
    pub messages: Vec<MessageIncludes>,
    pub subscriptions: Vec<UserSubscription>,
}

pub fn by_id(id: &Id) -> sql::ById<'_, threads::id> {
    threads::id.eq(id)
}

impl Thread {
    pub fn create(values: NewThread) -> sql::Insert<Table, NewThread> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        threads::table
    }
}
