use chrono::NaiveDateTime;
use diesel::deserialize::FromSqlRow;
use diesel::dsl::{AsSelect, Desc, Eq, InnerJoin, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::row::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::user::User;
use crate::models::message::Message;
use crate::schema::threads::{self, dsl as col};
use crate::schema::users;

pub type TableWithJoins = InnerJoin<threads::table, users::table>;
pub type DefaultSelection = (AsSelect<Thread, Pg>, AsSelect<User, Pg>);

#[derive(Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(User, foreign_key = owner_id))]
#[diesel(table_name = threads)]
#[diesel(check_for_backend(Pg))]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Uuid,

    name: String,

    owner_id: Uuid,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
#[serde(rename_all = "camelCase")]
pub struct NewThread {
    pub name: String,

    pub owner_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct ThreadDetails {
    #[serde(flatten)]
    pub thread: Thread,
    pub owner: User,
    pub messages: Vec<Message>,
}

#[derive(Serialize)]
pub struct ThreadWithOwner {
    #[serde(flatten)]
    pub thread: Thread,
    pub owner: User,
}

pub fn by_id(id: Uuid) -> Eq<col::id, Uuid> {
    col::id.eq(id)
}

pub fn created_at_desc() -> (Desc<col::created_at>, Desc<col::id>) {
    (col::created_at.desc(), col::id.desc())
}

impl Thread {
    pub const TABLE: threads::table = threads::table;

    pub fn query() -> Select<TableWithJoins, DefaultSelection> {
        Self::TABLE
            .inner_join(users::table)
            .select((Self::as_select(), User::as_select()))
    }
}

impl FromSqlRow<DefaultSelection, Pg> for ThreadWithOwner {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (thread, owner) = <_ as FromSqlRow<DefaultSelection, _>>::build_from_row(row)?;
        Ok(ThreadWithOwner { thread, owner })
    }
}
