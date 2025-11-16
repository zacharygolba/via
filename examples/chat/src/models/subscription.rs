use bitflags::bitflags;
use chrono::NaiveDateTime;
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::dsl::{Desc, InnerJoin};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use serde::{Deserialize, Serialize};

use super::{Thread, User};
use crate::models::user::UserPreview;
use crate::schema::{subscriptions, threads, users};
use crate::util::sql::{self, Id};

type Pk = subscriptions::id;
type Table = subscriptions::table;

type CreatedAtDesc = (Desc<subscriptions::created_at>, Desc<Pk>);
type CanParticipate = has_flags<subscriptions::claims, i32>;

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

diesel::define_sql_function! {
    /// SQL: (lhs & rhs) = rhs
    fn has_flags(lhs: Integer, rhs: Integer) -> Bool;
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Thread))]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    id: Id,
    claims: AuthClaims,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,

    #[serde(skip)]
    user_id: Id,

    #[serde(skip)]
    thread_id: Id,
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

#[derive(Clone, Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct ThreadSubscription {
    #[diesel(embed)]
    #[serde(flatten)]
    subscription: Subscription,

    #[diesel(embed)]
    thread: Thread,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct UserSubscription {
    #[diesel(embed)]
    #[serde(flatten)]
    pub subscription: Subscription,

    #[diesel(embed)]
    pub user: UserPreview,
}

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    subscriptions::id.eq(id)
}

pub fn by_thread(id: &Id) -> sql::ById<'_, subscriptions::thread_id> {
    subscriptions::thread_id.eq(id)
}

pub fn by_user(id: &Id) -> sql::ById<'_, subscriptions::user_id> {
    subscriptions::user_id.eq(id)
}

pub fn claims_can_participate() -> CanParticipate {
    has_flags(subscriptions::claims, AuthClaims::participate().bits())
}

pub fn created_at_desc() -> CreatedAtDesc {
    (subscriptions::created_at.desc(), subscriptions::id.desc())
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
    pub fn create(values: NewSubscription) -> sql::Insert<Table, NewSubscription> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        subscriptions::table
    }

    pub fn threads() -> InnerJoin<Table, threads::table> {
        Self::table().inner_join(Thread::table())
    }

    pub fn users() -> InnerJoin<Table, users::table> {
        Self::table().inner_join(User::table())
    }
}

impl ThreadSubscription {
    pub fn id(&self) -> Id {
        self.subscription.id
    }

    pub fn claims(&self) -> &AuthClaims {
        &self.subscription.claims
    }

    pub fn thread(&self) -> &Thread {
        &self.thread
    }

    pub fn user_id(&self) -> &Id {
        &self.subscription.user_id
    }

    pub fn foreign_keys(&self) -> (Id, Id) {
        (*self.user_id(), *self.thread.id())
    }
}
