use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::message::MessageWithJoins;
use crate::models::user::UserPreview;
use crate::schema::threads;
use crate::util::{Id, sql};

type Pk = threads::id;
type Table = threads::table;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Id,
    name: String,
    created_at: DateTime<Utc>,
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
pub struct ThreadWithJoins {
    #[serde(flatten)]
    pub thread: Thread,
    pub users: Vec<UserPreview>,
    pub messages: Vec<MessageWithJoins>,
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
