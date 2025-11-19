pub use crate::schema::subscriptions;

use bitflags::bitflags;
use chrono::{DateTime, Utc};
use diesel::AsExpression;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::dsl::InnerJoin;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use serde::{Deserialize, Serialize};

use crate::models::thread::{Thread, threads};
use crate::models::user::{UserPreview, users};
use crate::util::Id;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    id: Id,
    claims: AuthClaims,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    #[serde(skip)]
    user_id: Id,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = subscriptions)]
#[serde(rename_all = "camelCase")]
pub struct NewSubscription {
    pub user_id: Id,
    pub thread_id: Option<Id>,
    pub claims: AuthClaims,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = subscriptions)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    user_id: Id,
    claims: AuthClaims,
}

#[derive(Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = subscriptions)]
#[diesel(check_for_backend(Pg))]
pub struct ThreadSubscription {
    #[diesel(embed)]
    #[serde(flatten)]
    subscription: Subscription,

    #[diesel(embed)]
    thread: Thread,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = subscriptions)]
#[diesel(check_for_backend(Pg))]
pub struct UserSubscription {
    #[diesel(embed)]
    #[serde(flatten)]
    subscription: Subscription,

    #[diesel(embed)]
    user: UserPreview,
}

bitflags! {
    #[derive(AsExpression, Clone, Debug, Deserialize, FromSqlRow, Serialize)]
    #[diesel(sql_type = Integer)]
    pub struct AuthClaims: i32 {
        const VIEW        = 1 << 0;
        const WRITE       = 1 << 1;
        const REACT       = 1 << 2;
        const INVITE      = 1 << 3;
        const MODERATE    = 1 << 4;
    }
}

filters! {
    pub fn subscriptions::by_id(id == &Id);
    pub fn subscriptions::by_user(user_id == &Id);
    pub fn subscriptions::by_thread(thread_id == &Id);
}

sorts! {
    pub fn subscriptions::recent(#[desc] created_at, id);
}

diesel::define_sql_function! {
    /// SQL: (lhs & rhs) = rhs
    fn has_flags(lhs: Integer, rhs: Integer) -> Bool;
}

pub fn claims_can_participate() -> has_flags<subscriptions::claims, i32> {
    let participate = AuthClaims::VIEW | AuthClaims::WRITE | AuthClaims::REACT;
    has_flags(subscriptions::claims, participate.bits())
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
    pub fn query() -> subscriptions::table {
        subscriptions::table
    }

    pub fn threads() -> InnerJoin<subscriptions::table, threads::table> {
        Self::query().inner_join(threads::table)
    }

    pub fn users() -> InnerJoin<subscriptions::table, users::table> {
        Self::query().inner_join(users::table)
    }
}

impl NewSubscription {
    /// Subscribe user_id to thread_id with all auth claims.
    ///
    pub fn admin(user_id: Id, thread_id: Id) -> Self {
        Self {
            user_id,
            thread_id: Some(thread_id),
            claims: AuthClaims::all(),
        }
    }
}

impl ThreadSubscription {
    pub fn id(&self) -> &Id {
        &self.subscription.id
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
}
