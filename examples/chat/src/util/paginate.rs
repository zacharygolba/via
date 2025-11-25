use chrono::{DateTime, Utc};
use diesel::Expression;
use diesel::dsl::Offset;
use diesel::query_dsl::methods::{LimitDsl, OffsetDsl};
use diesel::sql_types::{Timestamptz, Uuid};
use via::request::QueryParams;
use via::{Error, raise};

use super::Id;

const INVALID_DATE_TIME: &str = "invalid datetime for keyset in query \"before\".";
const INVALID_KEYSET: &str = "invalid keyset in query \"before\".";
const INVALID_UUID: &str = "invalid uuid for keyset in query \"before\".";

const MIN_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 50;

pub const PER_PAGE: i64 = 25;

// type KeysetAfter<'a, A, B> = after_keyset<&'a DateTime<Utc>, &'a Id, A, B>;
type KeysetBefore<A, B> = after_keyset<A, B, DateTime<Utc>, Id>;

pub trait Paginate<T> {
    type Output;
    fn paginate(self, page: T) -> Self::Output;
}

#[derive(Debug)]
pub struct Keyset {
    pub limit: i64,
    pub value: (DateTime<Utc>, Id),
}

#[derive(Debug)]
pub struct Page {
    limit: i64,
    offset: i64,
}

diesel::define_sql_function! {
    /// SQL: (lhs0, lhs1) > (rhs0, rhs1)
    fn after_keyset(lhs0: Timestamptz, lhs1: Uuid, rhs0: Timestamptz, rhs1: Uuid) -> Bool;
}

fn limit_from_query(query: &QueryParams) -> Result<i64, Error> {
    let limit = query
        .first("limit")
        .optional()?
        .map_or(Ok(MIN_PER_PAGE), |value| value.parse())?
        .clamp(MIN_PER_PAGE, MAX_PER_PAGE);

    Ok(limit)
}

impl Keyset {
    // pub fn after<A, B>(&self, pivot: (A, B)) -> KeysetAfter<A, B>
    // where
    //     A: Expression<SqlType = Timestamptz>,
    //     B: Expression<SqlType = Uuid>,
    // {
    //     keyset_expr(self.value.0, self.value.1, pivot.0, pivot.1)
    // }

    pub fn before<A, B>(&self, pivot: (A, B)) -> KeysetBefore<A, B>
    where
        A: Expression<SqlType = Timestamptz>,
        B: Expression<SqlType = Uuid>,
    {
        let (timestamp, pk) = self.value;
        after_keyset(pivot.0, pivot.1, timestamp, pk)
    }
}

impl TryFrom<QueryParams<'_>> for Keyset {
    type Error = Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        let value = query
            .first("after")
            .decode()
            .into_result()
            .and_then(|value| {
                let mut parts = value.split(',');
                let Some((created_at, id)) = parts.next().zip(parts.next()) else {
                    raise!(400, message = INVALID_KEYSET);
                };

                match (created_at.parse(), id.parse()) {
                    (Ok(datetime), Ok(uuid)) => Ok((datetime, uuid)),
                    (Err(_), _) => raise!(400, message = INVALID_DATE_TIME),
                    _ => raise!(400, message = INVALID_UUID),
                }
            })?;

        Ok(Self {
            limit: limit_from_query(&query)?,
            value,
        })
    }
}

impl<T> Paginate<Page> for T
where
    T: LimitDsl,
    <T as LimitDsl>::Output: OffsetDsl,
{
    type Output = Offset<<T as LimitDsl>::Output>;

    fn paginate(self, page: Page) -> Self::Output {
        self.limit(page.limit).offset(page.offset)
    }
}

impl TryFrom<QueryParams<'_>> for Page {
    type Error = Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        let limit = limit_from_query(&query)?;
        let page = query.first("page").optional()?.map_or(Ok(1), |value| {
            value.parse::<i64>().or_else(|error| raise!(400, error))
        })?;

        if page < 1 {
            raise!(400, message = "page must be a positive integer.");
        }

        Ok(Self {
            limit,
            offset: (page - 1).saturating_mul(limit),
        })
    }
}
