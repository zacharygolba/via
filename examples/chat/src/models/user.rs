use chrono::NaiveDateTime;
use diesel::dsl::{AsSelect, Desc, Eq, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::users;
use crate::util::sql::{self, Id};

type Pk = users::id;
type Table = users::table;

type CreatedAtDesc = (Desc<users::created_at>, Desc<Pk>);

type SelectSelf = AsSelect<User, Pg>;

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

impl User {
    pub fn as_delete() -> Pk {
        users::id
    }

    pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
        users::id.eq(id)
    }

    pub fn by_username(username: &str) -> Eq<users::username, &str> {
        users::username.eq(username)
    }

    pub fn created_at_desc() -> CreatedAtDesc {
        (users::created_at.desc(), users::id.desc())
    }

    pub fn create(values: NewUser) -> sql::Insert<Table, NewUser> {
        diesel::insert_into(users::table).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(users::table)
            .filter(Self::by_id(id))
            .set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(users::table).filter(Self::by_id(id))
    }

    pub fn select() -> Select<Table, SelectSelf> {
        users::table.select(Self::as_select())
    }
}
