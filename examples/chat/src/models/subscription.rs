use bitflags::bitflags;
use chrono::NaiveDateTime;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::dsl::{self, AsSelect, Filter, InnerJoin, Select, Update, Values};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::query_builder::{DeleteStatement, IncompleteInsertStatement, IntoUpdateTarget};
use diesel::row::Row;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Integer;
use diesel::{AsExpression, define_sql_function};
use serde::{Deserialize, Serialize};

use crate::models::{Thread, User};
use crate::schema::subscriptions::{self, dsl as col};
use crate::schema::users;
use crate::util::Id;

pub type ById<'a> = dsl::Eq<col::id, &'a Id>;
pub type ByJoin<'a> = dsl::And<ByUser<'a>, ByThread<'a>>;
pub type ByUser<'a> = dsl::Eq<col::user_id, &'a Id>;
pub type ByThread<'a> = dsl::Eq<col::thread_id, &'a Id>;

pub type SelectSelf = AsSelect<Subscription, Pg>;
pub type SelectSelfWithUser = (AsSelect<Subscription, Pg>, AsSelect<User, Pg>);

type JoinUser = InnerJoin<subscriptions::table, users::table>;

/// The concrete return type of [`Subscription::create`].
///
type CreateSelf = Values<IncompleteInsertStatement<subscriptions::table>, NewSubscription>;

/// The concrete return type of [`Subscription::update`].
///
type UpdateSelf<'a> = Update<Filter<subscriptions::table, ById<'a>>, ChangeSet>;

/// The concrete return type of [`Subscription::delete`].
///
type DeleteSelf<'a> = DeleteStatement<
    subscriptions::table,
    <Filter<subscriptions::table, ById<'a>> as IntoUpdateTarget>::WhereClause,
>;

bitflags! {
    #[derive(AsExpression, Clone, Copy, Debug, Deserialize, FromSqlRow, Serialize)]
    #[diesel(sql_type = Integer)]
    pub struct AuthClaims: i32 {
        const VIEW            = 1 << 0;
        const WRITE           = 1 << 1;
        const REACT           = 1 << 2;
        const INVITE          = 1 << 3;
        const MODERATE        = 1 << 4;
    }
}

define_sql_function! {
    /// SQL: (lhs & rhs) = rhs
    fn has_flags(lhs: Integer, rhs: Integer) -> Bool;
}

#[derive(Associations, Clone, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Thread))]
#[diesel(table_name = subscriptions)]
#[diesel(primary_key(user_id, thread_id))]
#[diesel(check_for_backend(Pg))]
pub struct Subscription {
    pub id: Id,

    #[serde(skip)]
    pub user_id: Id,

    #[serde(skip)]
    pub thread_id: Id,

    claims: AuthClaims,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
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
pub struct ChangeSet {
    pub user_id: Id,
    pub claims: AuthClaims,
}

#[derive(Serialize)]
pub struct UserSubscription {
    #[serde(flatten)]
    pub subscription: Subscription,
    pub user: User,
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
    pub fn by_id(id: &Id) -> ById<'_> {
        col::id.eq(id)
    }

    pub fn by_join<'a>(user_id: &'a Id, thread_id: &'a Id) -> ByJoin<'a> {
        Self::by_user(user_id).and(Self::by_thread(thread_id))
    }

    pub fn by_thread(thread_id: &Id) -> ByThread<'_> {
        col::thread_id.eq(thread_id)
    }

    pub fn by_user(user_id: &Id) -> ByUser<'_> {
        col::user_id.eq(user_id)
    }

    pub fn user_can_participate() -> has_flags<subscriptions::claims, i32> {
        has_flags(
            subscriptions::claims,
            (AuthClaims::VIEW | AuthClaims::WRITE | AuthClaims::REACT).bits(),
        )
    }

    pub fn select() -> Select<subscriptions::table, SelectSelf> {
        subscriptions::table.select(Self::as_select())
    }

    pub fn join_user() -> Select<JoinUser, SelectSelfWithUser> {
        subscriptions::table
            .inner_join(users::table)
            .select((Self::as_select(), User::as_select()))
    }

    pub fn create(new_subscription: NewSubscription) -> CreateSelf {
        diesel::insert_into(subscriptions::table).values(new_subscription)
    }

    pub fn update(id: &Id, change_set: ChangeSet) -> UpdateSelf<'_> {
        diesel::update(subscriptions::table)
            .filter(Self::by_id(id))
            .set(change_set)
    }

    pub fn delete(id: &Id) -> DeleteSelf<'_> {
        diesel::delete(subscriptions::table).filter(Self::by_id(id))
    }

    pub fn claims(&self) -> &AuthClaims {
        &self.claims
    }

    pub fn thread_id(&self) -> &Id {
        &self.thread_id
    }

    pub fn user_id(&self) -> &Id {
        &self.user_id
    }
}

impl FromSqlRow<SelectSelfWithUser, Pg> for UserSubscription {
    fn build_from_row<'a>(row: &impl Row<'a, Pg>) -> diesel::deserialize::Result<Self> {
        let (subscription, user) = <_ as FromSqlRow<SelectSelfWithUser, _>>::build_from_row(row)?;
        Ok(UserSubscription { subscription, user })
    }
}
