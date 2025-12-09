use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::VarChar;
use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize, Serializer};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug)]
pub struct InvalidEmojiError;

#[derive(AsExpression, Clone, Debug, FromSqlRow)]
#[diesel(sql_type = VarChar)]
pub struct Emoji {
    buf: [u8; 16],
    len: usize,
}

impl Error for InvalidEmojiError {}

impl Display for InvalidEmojiError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "emoji exceeds max length.")
    }
}

impl FromStr for Emoji {
    type Err = InvalidEmojiError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut emoji = Self {
            buf: [0; 16],
            len: input.len(),
        };

        if emoji.len > 16 {
            Err(InvalidEmojiError)
        } else {
            emoji.buf[..emoji.len].copy_from_slice(input.as_bytes());
            Ok(emoji)
        }
    }
}

impl Deref for Emoji {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let slice = &self.buf[..self.len];
        // Safety: The bytes in self are guarenteed to be UTF-8.
        unsafe { str::from_utf8_unchecked(slice) }
    }
}

impl<'de> Deserialize<'de> for Emoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: &str = Deserialize::deserialize(deserializer)?;
        value.parse().map_err(D::Error::custom)
    }
}

impl FromSql<VarChar, Pg> for Emoji {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(str::from_utf8(value.as_bytes())?.parse()?)
    }
}

impl Serialize for Emoji {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

impl ToSql<VarChar, Pg> for Emoji {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        <str as ToSql<VarChar, Pg>>::to_sql(self, out)
    }
}
