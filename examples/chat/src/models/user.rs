pub use crate::schema::users;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::Id;

#[derive(Clone, Deserialize, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    id: Id,
    email: String,
    username: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    email: String,
    username: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = users)]
pub struct ChangeSet {
    email: Option<String>,
    username: Option<String>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = users)]
pub struct UserPreview {
    id: Id,
    username: String,
}

filters! {
    pub fn users::by_id(id == &Id);
    pub fn users::by_username(username == &str);
}

sorts! {
    pub fn users::recent(#[desc] created_at, id);
}

impl User {
    pub fn query() -> users::table {
        users::table
    }
}
