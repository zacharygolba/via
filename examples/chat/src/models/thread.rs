use chrono::NaiveDateTime;
use diesel::dsl::{self, AsSelect, Desc, InnerJoinOn, Select};
use diesel::pg::Pg;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::message::MessageIncludes;
use crate::models::subscription::UserSubscription;
use crate::schema::{subscriptions, threads};
use crate::util::sql::{self, Id};

type Pk = threads::id;
type Table = threads::table;

type CreatedAtDesc = (Desc<threads::created_at>, Desc<Pk>);

type SelectSelf = AsSelect<Thread, Pg>;
type ThroughSubscriptions =
    InnerJoinOn<Table, subscriptions::table, dsl::Eq<Pk, subscriptions::thread_id>>;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(table_name = threads)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: Id,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
pub struct NewThread {
    name: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = threads)]
pub struct ChangeSet {
    name: String,
}

#[derive(Serialize)]
pub struct ThreadIncludes {
    #[serde(flatten)]
    pub thread: Thread,
    pub messages: Vec<MessageIncludes>,
    pub subscriptions: Vec<UserSubscription>,
}

impl Thread {
    pub fn as_delete() -> Pk {
        threads::id
    }

    pub fn by_id(id: &Id) -> sql::ById<'_, threads::id> {
        threads::id.eq(id)
    }

    pub fn created_at_desc() -> CreatedAtDesc {
        (threads::created_at.desc(), threads::id.desc())
    }

    pub fn create(values: NewThread) -> sql::Insert<Table, NewThread> {
        diesel::insert_into(threads::table).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(threads::table)
            .filter(Self::by_id(id))
            .set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(threads::table).filter(Self::by_id(id))
    }

    pub fn select() -> Select<Table, SelectSelf> {
        threads::table.select(Self::as_select())
    }

    pub fn subscriptions() -> Select<ThroughSubscriptions, SelectSelf> {
        let on = threads::id.eq(subscriptions::thread_id);

        threads::table
            .inner_join(subscriptions::table.on(on))
            .select(Self::as_select())
    }
}
