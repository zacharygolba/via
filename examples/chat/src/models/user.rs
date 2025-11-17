use chrono::{DateTime, Utc};
use diesel::dsl::{self, Desc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::users;
use crate::util::sql::{self, Id};

type Pk = users::id;
type Table = users::table;

type CreatedAtDesc = (Desc<users::created_at>, Desc<Pk>);

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

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    users::id.eq(id)
}

pub fn by_username(username: &str) -> dsl::Eq<users::username, &str> {
    users::username.eq(username)
}

pub fn created_at_desc() -> CreatedAtDesc {
    (users::created_at.desc(), users::id.desc())
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

impl From<&'_ User> for UserPreview {
    fn from(user: &'_ User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
        }
    }
}
