use chrono::NaiveDateTime;
use diesel::deserialize::FromSqlRow;
use diesel::dsl::{self, AsSelect, Desc, Filter, Order};
use diesel::helper_types::{InnerJoin, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::row::Row;
use serde::{Deserialize, Serialize};

use super::{Thread, User};
use crate::models::user::UserPreview;
use crate::schema::{messages, users};
use crate::util::sql::{self, Id};

type Pk = messages::id;
type Table = messages::table;

type CreatedAtDesc = (Desc<messages::created_at>, Desc<Pk>);
type InThread<'a, T> = Order<Filter<T, sql::ById<'a, messages::thread_id>>, CreatedAtDesc>;

type SelectSelf = AsSelect<Message, Pg>;
type SelectIncludes = (SelectSelf, AsSelect<UserPreview, Pg>);

type Includes = InnerJoin<Table, users::table>;

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(Thread))]
#[diesel(belongs_to(User, foreign_key = author_id))]
#[diesel(table_name = messages)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    id: Id,
    body: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,

    #[serde(skip)]
    author_id: Id,

    #[serde(skip)]
    pub thread_id: Id,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = messages)]
pub struct ChangeSet {
    pub body: String,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = messages)]
#[serde(rename_all = "camelCase")]
pub struct NewMessage {
    pub body: String,
    pub author_id: Option<Id>,
    pub thread_id: Option<Id>,
}

#[derive(Serialize)]
pub struct MessageIncludes {
    #[serde(flatten)]
    message: Message,
    author: UserPreview,
}

impl Message {
    pub fn as_delete() -> Pk {
        messages::id
    }

    pub fn as_includes() -> SelectIncludes {
        (Self::as_select(), UserPreview::as_select())
    }

    pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
        messages::id.eq(id)
    }

    pub fn by_author_id(id: &Id) -> sql::ById<'_, messages::author_id> {
        messages::author_id.eq(id)
    }

    pub fn created_at_desc() -> CreatedAtDesc {
        (messages::created_at.desc(), messages::id.desc())
    }

    pub fn created_before(before: &NaiveDateTime) -> dsl::Gt<messages::created_at, &NaiveDateTime> {
        messages::created_at.gt(before)
    }

    pub fn create(values: NewMessage) -> sql::Insert<Table, NewMessage> {
        diesel::insert_into(messages::table).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(messages::table)
            .filter(Self::by_id(id))
            .set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(messages::table).filter(Self::by_id(id))
    }

    pub fn select() -> Select<Table, SelectSelf> {
        messages::table.select(Self::as_select())
    }

    pub fn includes() -> Select<Includes, SelectIncludes> {
        messages::table
            .inner_join(users::table)
            .select(Self::as_includes())
    }

    pub fn in_thread(id: &Id) -> InThread<'_, Select<Includes, SelectIncludes>> {
        Self::includes()
            .filter(messages::thread_id.eq(id))
            .order(Self::created_at_desc())
    }
}

impl FromSqlRow<SelectIncludes, Pg> for MessageIncludes {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (message, author) = <_ as FromSqlRow<SelectIncludes, _>>::build_from_row(row)?;
        Ok(MessageIncludes { message, author })
    }
}
