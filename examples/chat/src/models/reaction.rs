use chrono::NaiveDateTime;
use diesel::deserialize::FromSqlRow;
use diesel::dsl::{AsSelect, Desc, Filter};
use diesel::helper_types::{InnerJoin, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::row::Row;
use serde::{Deserialize, Serialize};

use super::{Message, User};
use crate::schema::{reactions, users};
use crate::util::sql::{self, Id};

type Pk = reactions::id;
type Table = reactions::table;

type CreatedAtDesc = (Desc<reactions::created_at>, Desc<Pk>);
type ToMessage<'a, T> = Filter<T, sql::ById<'a, reactions::message_id>>;

type SelectSelf = AsSelect<Reaction, Pg>;
type SelectIncludes = (SelectSelf, AsSelect<User, Pg>);

type Includes = InnerJoin<Table, users::table>;

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Message))]
#[diesel(belongs_to(User))]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Id,
    emoji: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,

    #[serde(skip)]
    message_id: Id,

    #[serde(skip)]
    user_id: Id,
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
pub struct ReactionIncludes {
    #[serde(flatten)]
    pub reaction: Reaction,
    pub user: User,
}

impl Reaction {
    pub fn as_delete() -> Pk {
        reactions::id
    }

    pub fn as_includes() -> SelectIncludes {
        (Self::as_select(), User::as_select())
    }

    pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
        reactions::id.eq(id)
    }

    pub fn by_user_id(id: &Id) -> sql::ById<'_, reactions::user_id> {
        reactions::user_id.eq(id)
    }

    pub fn created_at_desc() -> CreatedAtDesc {
        (reactions::created_at.desc(), reactions::id.desc())
    }

    pub fn create(values: NewReaction) -> sql::Insert<Table, NewReaction> {
        diesel::insert_into(reactions::table).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(reactions::table)
            .filter(Self::by_id(id))
            .set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(reactions::table).filter(Self::by_id(id))
    }

    pub fn select() -> Select<Table, SelectSelf> {
        reactions::table.select(Self::as_select())
    }

    pub fn includes() -> Select<Includes, SelectIncludes> {
        reactions::table
            .inner_join(users::table)
            .select(Self::as_includes())
    }

    pub fn to_message(id: &Id) -> ToMessage<'_, Select<Includes, SelectIncludes>> {
        Self::includes().filter(reactions::message_id.eq(id))
    }
}

impl FromSqlRow<SelectIncludes, Pg> for ReactionIncludes {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (reaction, user) = <_ as FromSqlRow<SelectIncludes, _>>::build_from_row(row)?;

        Ok(ReactionIncludes { reaction, user })
    }
}
