use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::conversation::ConversationDetails;
use crate::models::user::UserPreview;
use crate::schema::channels;
use crate::util::Id;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    id: Id,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = channels)]
pub struct NewChannel {
    name: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = channels)]
pub struct ChangeSet {
    name: String,
}

#[derive(Serialize)]
pub struct ChannelWithJoins {
    #[serde(flatten)]
    channel: Channel,
    users: Vec<UserPreview>,
    threads: Vec<ConversationDetails>,
}

filters! {
    pub fn by_id(id == &Id) on channels;
}

impl Channel {
    pub fn joins(
        self,
        users: Vec<UserPreview>,
        threads: Vec<ConversationDetails>,
    ) -> ChannelWithJoins {
        ChannelWithJoins {
            channel: self,
            threads,
            users,
        }
    }
}
