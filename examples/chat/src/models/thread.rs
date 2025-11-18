pub use crate::schema::threads;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::message::MessageWithJoins;
use crate::models::user::UserPreview;
use crate::util::Id;

#[derive(Clone, Identifiable, Queryable, Selectable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Id,
    name: String,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
pub struct NewThread {
    name: String,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = threads)]
pub struct ChangeSet {
    name: String,
}

#[derive(Serialize)]
pub struct ThreadWithJoins {
    #[serde(flatten)]
    thread: Thread,
    users: Vec<UserPreview>,
    messages: Vec<MessageWithJoins>,
}

filters! {
    by_id(id == &Id) on threads,
}

impl Thread {
    pub fn joins(
        self,
        users: Vec<UserPreview>,
        messages: Vec<MessageWithJoins>,
    ) -> ThreadWithJoins {
        ThreadWithJoins {
            thread: self,
            users,
            messages,
        }
    }
}
