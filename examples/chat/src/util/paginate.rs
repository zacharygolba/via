use chrono::NaiveDateTime;
use via::raise;
use via::request::QueryParams;

use super::Id;

pub struct Cursor(pub NaiveDateTime, pub Id);

pub struct LimitAndOffset(pub i64, pub i64);

impl TryFrom<QueryParams<'_>> for Cursor {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self(
            query.first("created-before").parse()?,
            query.first("occurs-before").parse()?,
        ))
    }
}

impl TryFrom<QueryParams<'_>> for LimitAndOffset {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        let Some((limit, offset)) = query
            .first("limit")
            .optional()
            .zip(query.first("offset").optional())
        else {
            return Ok(Self(25, 0));
        };

        // Try expressions will improve this.
        Ok(Self(
            limit?.parse().or_else(|error| raise!(400, error))?,
            offset?.parse().or_else(|error| raise!(400, error))?,
        ))
    }
}
