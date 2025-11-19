pub use crate::schema::users;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::{Id, sql};

type Pk = users::id;
type Table = users::table;

#[derive(Clone, Deserialize, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Id,

    pub email: String,
    pub username: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub email: String,
    pub username: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = users)]
pub struct ChangeSet {
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = users)]
pub struct UserPreview {
    pub id: Id,
    pub username: String,
}

filters! {
    pub fn users::by_id(id == &Id);
    pub fn users::by_username(username == &str);
}

sorts! {
    pub fn users::recent(#[desc] created_at, id);
}

impl User {
    pub fn create(values: NewUser) -> sql::Insert<Table, NewUser> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        users::table
    }
}
