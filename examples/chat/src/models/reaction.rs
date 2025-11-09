use chrono::NaiveDateTime;
use diesel::deserialize::FromSqlRow;
use diesel::dsl::{AsSelect, Desc, Eq};
use diesel::helper_types::{InnerJoin, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::row::Row;
use serde::{Deserialize, Serialize};

use super::message::Message;
use super::user::User;
use crate::schema::reactions::{self, dsl as col};
use crate::schema::users;
use crate::util::Id;

pub type TableWithJoins = InnerJoin<reactions::table, users::table>;
pub type DefaultSelection = (AsSelect<Reaction, Pg>, AsSelect<User, Pg>);

#[derive(Associations, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Message))]
#[diesel(belongs_to(User))]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Id,

    emoji: String,

    #[serde(skip)]
    message_id: Id,

    #[serde(skip)]
    user_id: Id,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = reactions)]
pub struct ChangeSet {
    pub emoji: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct NewReaction {
    pub emoji: String,

    pub message_id: Option<Id>,
    pub user_id: Option<Id>,
}

#[derive(Serialize)]
pub struct ReactionWithJoins {
    #[serde(flatten)]
    reaction: Reaction,
    user: User,
}

pub fn by_id(id: Id) -> Eq<col::id, Id> {
    col::id.eq(id)
}

pub fn by_message(id: Id) -> Eq<col::message_id, Id> {
    col::message_id.eq(id)
}

pub fn by_user(id: Id) -> Eq<col::user_id, Id> {
    col::user_id.eq(id)
}

pub fn created_at_desc() -> (Desc<col::created_at>, Desc<col::id>) {
    (col::created_at.desc(), col::id.desc())
}

impl Reaction {
    pub const TABLE: reactions::table = reactions::table;

    pub fn query() -> Select<TableWithJoins, DefaultSelection> {
        Self::TABLE
            .inner_join(users::table)
            .select((Self::as_select(), User::as_select()))
    }
}

impl FromSqlRow<DefaultSelection, Pg> for ReactionWithJoins {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (reaction, user) = <_ as FromSqlRow<DefaultSelection, _>>::build_from_row(row)?;

        Ok(ReactionWithJoins { reaction, user })
    }
}
