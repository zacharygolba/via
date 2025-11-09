use chrono::NaiveDateTime;
use diesel::dsl;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::result::Error;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};

use super::Connection;
use crate::schema::users::{self, dsl as col};
use crate::util::Id;

pub type AsSelect = diesel::dsl::AsSelect<User, Pg>;
pub type Select = diesel::helper_types::Select<users::table, AsSelect>;

#[derive(Clone, Deserialize, Queryable, Selectable, Serialize)]
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

#[derive(Deserialize)]
pub struct LoginParams {
    pub username: String,
}

pub fn by_username(username: &str) -> dsl::Eq<col::username, &str> {
    col::username.eq(username)
}

impl User {
    pub fn query() -> Select {
        users::table.select(Self::as_select())
    }

    pub async fn create(connection: &mut Connection<'_>, new_user: NewUser) -> Result<Self, Error> {
        diesel::insert_into(users::table)
            .values(new_user)
            .returning(Self::as_returning())
            .get_result(connection)
            .await
    }
}
