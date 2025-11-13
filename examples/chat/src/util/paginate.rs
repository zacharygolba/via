use chrono::NaiveDateTime;
use via::request::QueryParams;

pub struct Cursor {
    pub before: NaiveDateTime,
}

pub struct LimitAndOffset(pub i64, pub i64);

impl TryFrom<QueryParams<'_>> for Cursor {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            before: query.first("before").parse()?,
        })
    }
}

impl TryFrom<QueryParams<'_>> for LimitAndOffset {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        let limit = query.first("limit").parse().unwrap_or(25);
        let offset = query.first("offset").parse().unwrap_or(0);

        Ok(Self(limit, offset))
    }
}
