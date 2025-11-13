use chrono::NaiveDateTime;
use diesel::dsl::{self, AsSelect, Filter, Order, Select, Update, Values};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{DeleteStatement, IncompleteInsertStatement, IntoUpdateTarget};
use serde::{Deserialize, Serialize};

use crate::models::subscription::{self, Subscription};
use crate::schema::subscriptions;
use crate::schema::threads::{self, dsl as col};
use crate::util::Id;

/// The default ORDER expression used when querying the threads table.
///
type CreatedAtDesc = (dsl::Desc<col::created_at>, dsl::Desc<col::id>);

/// The default WHERE clause used when finding a thread by id.
///
type WhereIdEq<'a> = Filter<threads::table, dsl::Eq<threads::id, &'a Id>>;

/// The concrete return type of [`Thread::create`].
///
type CreateSelf = Values<IncompleteInsertStatement<threads::table>, NewThread>;

/// The concrete return type of [`Thread::update`].
///
type UpdateSelf<'a> = Update<WhereIdEq<'a>, ChangeSet>;

/// The concrete return type of [`Thread::delete`].
///
type DeleteSelf<'a> =
    DeleteStatement<threads::table, <WhereIdEq<'a> as IntoUpdateTarget>::WhereClause>;

/// The SELECT statement and FROM clause of a query to the threads table.
///
type SelectSelf = Select<
    dsl::InnerJoinOn<
        threads::table,
        subscriptions::table,
        dsl::Eq<subscriptions::thread_id, threads::id>,
    >,
    AsSelect<Thread, Pg>,
>;

/// The concrete return type of [`Thread::by_participant`].
///
type ByParticipant<'a> =
    Order<Filter<SelectSelf, dsl::Eq<subscriptions::user_id, &'a Id>>, CreatedAtDesc>;

/// The concrete return type of [`Thread::by_subscription`].
///
type BySubscription<'a> = Order<Filter<SelectSelf, subscription::ByJoin<'a>>, CreatedAtDesc>;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(User, foreign_key = owner_id))]
#[diesel(table_name = threads)]
#[diesel(check_for_backend(Pg))]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Id,
    name: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = threads)]
pub struct ChangeSet {
    name: String,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
pub struct NewThread {
    name: String,
}

impl Thread {
    pub fn create(new_thread: NewThread) -> CreateSelf {
        diesel::insert_into(threads::table).values(new_thread)
    }

    pub fn delete(id: &Id) -> DeleteSelf<'_> {
        diesel::delete(threads::table).filter(threads::id.eq(id))
    }

    pub fn update(id: &Id, change_set: ChangeSet) -> UpdateSelf<'_> {
        diesel::update(threads::table)
            .filter(threads::id.eq(id))
            .set(change_set)
    }

    pub fn by_participant(user_id: &Id) -> ByParticipant<'_> {
        threads::table
            .select(Self::as_select())
            .inner_join(subscriptions::table.on(subscriptions::thread_id.eq(threads::id)))
            .filter(subscriptions::user_id.eq(user_id))
            .order((col::created_at.desc(), col::id.desc()))
    }

    pub fn by_subscription(subscription: &Subscription) -> BySubscription<'_> {
        let user_id = subscription.user_id();
        let thread_id = subscription.thread_id();

        threads::table
            .select(Self::as_select())
            .inner_join(subscriptions::table.on(subscriptions::thread_id.eq(threads::id)))
            .filter(Subscription::by_join(user_id, thread_id))
            .order((col::created_at.desc(), col::id.desc()))
    }

    pub fn id(&self) -> &Id {
        &self.id
    }
}
