use chrono::NaiveDateTime;
use diesel::dsl::{AsSelect, Desc, Eq, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::users::{self, dsl as col};
use crate::util::Id;

pub type TableWithJoins = users::table;
pub type DefaultSelection = AsSelect<User, Pg>;

#[derive(Clone, Deserialize, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(table_name = users)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Id,

    pub email: String,
    pub username: String,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub email: String,
    pub username: String,
}

pub fn by_id(id: Id) -> Eq<col::id, Id> {
    col::id.eq(id)
}

pub fn by_username(username: &str) -> Eq<col::username, &str> {
    col::username.eq(username)
}

pub fn created_at_desc() -> (Desc<col::created_at>, Desc<col::id>) {
    (col::created_at.desc(), col::id.desc())
}

impl User {
    pub const TABLE: users::table = users::table;

    pub fn query() -> Select<TableWithJoins, DefaultSelection> {
        Self::TABLE.select(Self::as_select())
    }
}
