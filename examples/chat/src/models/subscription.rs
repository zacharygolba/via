use bitflags::bitflags;
use chrono::NaiveDateTime;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::dsl::{And, AsSelect, Desc, InnerJoin, Select};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::row::Row;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::{AsExpression, define_sql_function};
use serde::{Deserialize, Serialize};

use super::{Thread, User};
use crate::schema::{subscriptions, threads, users};
use crate::util::sql::{self, Id};

type Pk = subscriptions::id;
type Table = subscriptions::table;

type CreatedAtDesc = (Desc<subscriptions::created_at>, Desc<Pk>);
type CanParticipate = has_flags<subscriptions::claims, i32>;

type SelectSelf = AsSelect<Subscription, Pg>;
type SelectJoinThreads = (SelectSelf, AsSelect<Thread, Pg>);
type SelectJoinUsers = (SelectSelf, AsSelect<User, Pg>);

type JoinThreads = InnerJoin<Table, threads::table>;
type JoinUsers = InnerJoin<Table, users::table>;

type ByJoin<'a> =
    And<sql::ById<'a, subscriptions::user_id>, sql::ById<'a, subscriptions::thread_id>>;

bitflags! {
    #[derive(AsExpression, Clone, Copy, Debug, Deserialize, FromSqlRow, Serialize)]
    #[diesel(sql_type = Integer)]
    pub struct AuthClaims: i32 {
        const VIEW        = 1 << 0;
        const WRITE       = 1 << 1;
        const REACT       = 1 << 2;
        const INVITE      = 1 << 3;
        const MODERATE    = 1 << 4;
    }
}

define_sql_function! {
    /// SQL: (lhs & rhs) = rhs
    fn has_flags(lhs: Integer, rhs: Integer) -> Bool;
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Thread))]
#[diesel(table_name = subscriptions)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: Id,
    pub claims: AuthClaims,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    #[serde(skip)]
    pub user_id: Id,

    #[serde(skip)]
    pub thread_id: Id,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = subscriptions)]
#[serde(rename_all = "camelCase")]
pub struct NewSubscription {
    pub claims: AuthClaims,
    pub user_id: Id,
    pub thread_id: Option<Id>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = subscriptions)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub claims: AuthClaims,
    pub user_id: Id,
}

#[derive(Clone, Serialize)]
pub struct ThreadSubscription {
    #[serde(flatten)]
    pub subscription: Subscription,
    pub thread: Thread,
}

#[derive(Serialize)]
pub struct UserSubscription {
    #[serde(flatten)]
    pub subscription: Subscription,
    pub user: User,
}

impl AuthClaims {
    pub fn participate() -> Self {
        Self::VIEW | Self::WRITE | Self::REACT
    }
}

impl FromSql<Integer, Pg> for AuthClaims {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        i32::from_sql(bytes).map(Self::from_bits_truncate)
    }
}

impl ToSql<Integer, Pg> for AuthClaims {
    fn to_sql<'a>(&'a self, output: &mut Output<'a, '_, Pg>) -> serialize::Result {
        <_ as ToSql<Integer, _>>::to_sql(&self.bits(), &mut output.reborrow())
    }
}

impl Subscription {
    pub fn as_delete() -> Pk {
        subscriptions::id
    }

    pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
        subscriptions::id.eq(id)
    }

    pub fn by_join<'a>(user_id: &'a Id, thread_id: &'a Id) -> ByJoin<'a> {
        Self::by_user(user_id).and(Self::by_thread(thread_id))
    }

    pub fn by_thread(id: &Id) -> sql::ById<'_, subscriptions::thread_id> {
        subscriptions::thread_id.eq(id)
    }

    pub fn by_user(id: &Id) -> sql::ById<'_, subscriptions::user_id> {
        subscriptions::user_id.eq(id)
    }

    pub fn can_participate() -> CanParticipate {
        has_flags(subscriptions::claims, AuthClaims::participate().bits())
    }

    pub fn created_at_desc() -> CreatedAtDesc {
        (subscriptions::created_at.desc(), subscriptions::id.desc())
    }

    pub fn create(values: NewSubscription) -> sql::Insert<Table, NewSubscription> {
        diesel::insert_into(subscriptions::table).values(values)
    }

    pub fn update(id: &Id, change_set: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(subscriptions::table)
            .filter(Self::by_id(id))
            .set(change_set)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(subscriptions::table).filter(Self::by_id(id))
    }

    pub fn select() -> Select<Table, SelectSelf> {
        subscriptions::table.select(Self::as_select())
    }
}

impl NewSubscription {
    pub fn admin(user: &User, thread: &Thread) -> Self {
        Self {
            claims: AuthClaims::all(),
            user_id: user.id,
            thread_id: Some(thread.id),
        }
    }
}

impl ThreadSubscription {
    pub fn select() -> Select<JoinThreads, SelectJoinThreads> {
        subscriptions::table
            .inner_join(threads::table)
            .select((Subscription::as_select(), Thread::as_select()))
    }

    pub fn id(&self) -> &Id {
        &self.subscription.id
    }

    pub fn claims(&self) -> &AuthClaims {
        &self.subscription.claims
    }
}

impl UserSubscription {
    pub fn select() -> Select<JoinUsers, SelectJoinUsers> {
        subscriptions::table
            .inner_join(users::table)
            .select((Subscription::as_select(), User::as_select()))
    }
}

impl FromSqlRow<SelectJoinThreads, Pg> for ThreadSubscription {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (subscription, thread) = <_ as FromSqlRow<SelectJoinThreads, _>>::build_from_row(row)?;

        Ok(Self {
            subscription,
            thread,
        })
    }
}

impl FromSqlRow<SelectJoinUsers, Pg> for UserSubscription {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (subscription, user) = <_ as FromSqlRow<SelectJoinUsers, _>>::build_from_row(row)?;
        Ok(Self { subscription, user })
    }
}
