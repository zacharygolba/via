use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::dsl::Desc;
use diesel::expression::AsExpression;
use diesel::helper_types::InnerJoin;
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
use diesel::{prelude::*, serialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use super::User;
use super::message::{Message, MessageWithAuthor};
use crate::models::user::UserPreview;
use crate::schema::{reactions, users};
use crate::util::sql::{self, Id};

type Pk = reactions::id;
type Table = reactions::table;

type CreatedAtDesc = (Desc<reactions::created_at>, Desc<Pk>);

#[derive(Debug)]
pub struct InvalidEmojiError;

#[derive(AsExpression, Clone, Debug, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Emoji {
    buf: [u8; 16],
    len: usize,
}

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(MessageWithAuthor, foreign_key = message_id))]
#[diesel(belongs_to(Message))]
#[diesel(belongs_to(User))]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Id,
    emoji: Emoji,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,

    #[serde(skip)]
    message_id: Id,

    #[serde(skip)]
    user_id: Id,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = reactions)]
pub struct ChangeSet {
    pub emoji: Emoji,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = reactions)]
#[serde(rename_all = "camelCase")]
pub struct NewReaction {
    pub emoji: Emoji,
    pub message_id: Option<Id>,
    pub user_id: Option<Id>,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(check_for_backend(Pg))]
pub struct ReactionIncludes {
    #[diesel(embed)]
    #[serde(flatten)]
    reaction: Reaction,

    #[diesel(embed)]
    user: UserPreview,
}

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(belongs_to(MessageWithAuthor, foreign_key = message_id))]
#[diesel(table_name = reactions)]
#[diesel(check_for_backend(Pg))]
pub struct ReactionPreview {
    id: Id,
    emoji: Emoji,

    #[diesel(embed)]
    user: UserPreview,

    #[serde(skip)]
    message_id: Id,
}

pub fn by_id(id: &Id) -> sql::ById<'_, Pk> {
    reactions::id.eq(id)
}

pub fn by_message(id: &Id) -> sql::ById<'_, reactions::message_id> {
    reactions::message_id.eq(id)
}

pub fn by_user(id: &Id) -> sql::ById<'_, reactions::user_id> {
    reactions::user_id.eq(id)
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

    fn deref(&self) -> &Self::Target {
        let slice = &self.buf[..self.len];

        // Safety: The bytes in self are guarenteed to be UTF-8.
        unsafe { str::from_utf8_unchecked(slice) }
    }
}

impl FromSql<Text, Pg> for Emoji {
    fn from_sql(value: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(str::from_utf8(value.as_bytes())?.parse()?)
    }
}

impl ToSql<Text, Pg> for Emoji {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        <str as ToSql<Text, Pg>>::to_sql(self, out)
    }
}

impl<'de> Deserialize<'de> for Emoji {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        <&'de str as Deserialize>::deserialize(deserializer)?
            .parse::<Emoji>()
            .map_err(|error| D::Error::custom(error.to_string()))
    }
}

impl Serialize for Emoji {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

impl Error for InvalidEmojiError {}

impl Display for InvalidEmojiError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "emoji exceeds max length.")
    }
}

impl Reaction {
    pub const ID: Pk = reactions::id;

    pub fn created_at_desc() -> CreatedAtDesc {
        (reactions::created_at.desc(), reactions::id.desc())
    }

    pub fn create(values: NewReaction) -> sql::Insert<Table, NewReaction> {
        diesel::insert_into(Self::table()).values(values)
    }

    pub fn update(id: &Id, changes: ChangeSet) -> sql::Update<'_, Table, Pk, ChangeSet> {
        diesel::update(Self::table()).filter(by_id(id)).set(changes)
    }

    pub fn delete(id: &Id) -> sql::Delete<'_, Table, Pk> {
        diesel::delete(Self::table()).filter(by_id(id))
    }

    pub fn table() -> Table {
        reactions::table
    }

    pub fn includes() -> InnerJoin<Table, users::table> {
        Self::table().inner_join(User::table())
    }
}
