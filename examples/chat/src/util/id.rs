use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

use super::error::InvalidIdError;

#[derive(
    AsExpression,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    FromSqlRow,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
)]
#[diesel(sql_type = sql_types::Uuid)]
pub struct Id(Uuid);

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        AsRef::as_ref(&self.0)
    }
}

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

impl<'a> TryFrom<&'a [u8]> for Id {
    type Error = InvalidIdError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        if let Ok(uuid) = Uuid::from_slice(value) {
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
