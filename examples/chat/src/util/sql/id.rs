use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

#[derive(
    AsExpression, Clone, Copy, Debug, Deserialize, Eq, FromSqlRow, Hash, PartialEq, Serialize,
)]
#[diesel(sql_type = sql_types::Uuid)]
pub struct Id(Uuid);

#[derive(Debug)]
pub struct InvalidIdError;

impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for Id {
    type Err = InvalidIdError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Ok(uuid) = input.parse() {
            Ok(Self(uuid))
        } else {
            Err(InvalidIdError)
        }
    }
}

impl FromSql<sql_types::Uuid, Pg> for Id {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(Self(Uuid::from_sql(bytes)?))
    }
}

impl<DB: Backend> ToSql<sql_types::Uuid, DB> for Id
where
    Uuid: ToSql<sql_types::Uuid, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl Error for InvalidIdError {}

impl Display for InvalidIdError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid uuid")
    }
}
