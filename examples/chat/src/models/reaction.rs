use chrono::NaiveDateTime;
use diesel::dsl::Desc;
use diesel::helper_types::InnerJoin;
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Message, User};
use crate::models::user::UserPreview;
use crate::schema::{reactions, users};
use crate::util::sql::{self, Id};

type Pk = reactions::id;
type Table = reactions::table;

type CreatedAtDesc = (Desc<reactions::created_at>, Desc<Pk>);

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

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct ReactionIncludes {
    #[diesel(embed)]
    #[serde(flatten)]
    reaction: Reaction,

    #[diesel(embed)]
    user: UserPreview,
}

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    reactions::id.eq(id)
}

pub fn by_message(id: &Id) -> sql::ById<'_, reactions::message_id> {
    reactions::message_id.eq(id)
}

pub fn by_user(id: &Id) -> sql::ById<'_, reactions::user_id> {
    reactions::user_id.eq(id)
}

pub fn created_at_desc() -> CreatedAtDesc {
    (reactions::created_at.desc(), reactions::id.desc())
}

impl Reaction {
    pub fn create(values: NewReaction) -> sql::Insert<Table, NewReaction> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        reactions::table
    }

    pub fn includes() -> InnerJoin<Table, users::table> {
        Self::table().inner_join(User::table())
    }
}
