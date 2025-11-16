use diesel::dsl::Offset;
use diesel::query_dsl::methods::{LimitDsl, OffsetDsl};
use via::raise;
use via::request::QueryParams;

const MIN_PER_PAGE: i64 = 25;
const MAX_PER_PAGE: i64 = 50;

#[derive(Debug)]
pub struct PageAndLimit {
    limit: i64,
    offset: i64,
}

pub trait Paginate<Page> {
    type Output;
    fn paginate(self, page: Page) -> Self::Output;
}

impl<T> Paginate<PageAndLimit> for T
where
    T: LimitDsl,
    <T as LimitDsl>::Output: OffsetDsl,
{
    type Output = Offset<<T as LimitDsl>::Output>;

    fn paginate(self, page: PageAndLimit) -> Self::Output {
        self.limit(page.limit).offset(page.offset)
    }
}

impl TryFrom<QueryParams<'_>> for PageAndLimit {
    type Error = via::Error;

    fn try_from(query: QueryParams<'_>) -> Result<Self, Self::Error> {
        let page = query.first("page").optional()?.map_or(Ok(1), |value| {
            value.parse::<i64>().or_else(|error| raise!(400, error))
        })?;

        let limit = query
            .first("limit")
            .optional()?
            .map_or(Ok(MIN_PER_PAGE), |value| value.parse())?
            .clamp(MIN_PER_PAGE, MAX_PER_PAGE);

        if page < 1 {
            raise!(400, message = "page must be a positive integer.");
        }

        Ok(Self {
            limit,
            offset: (page - 1).saturating_mul(limit),
        })
    }
}
