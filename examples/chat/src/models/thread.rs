use chrono::NaiveDateTime;
use diesel::result::Error;
use diesel::{pg::Pg, prelude::*};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Connection;
use crate::schema::threads;

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = threads)]
#[diesel(check_for_backend(Pg))]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    id: Uuid,

    name: String,

    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Clone, Deserialize, Insertable)]
#[diesel(table_name = threads)]
#[serde(rename_all = "camelCase")]
pub struct NewThread {
    pub name: String,

    pub owner_id: Option<Uuid>,
}

impl Thread {
    pub async fn create(
        connection: &mut Connection<'_>,
        new_thread: NewThread,
    ) -> Result<Self, Error> {
        diesel::insert_into(threads::table)
            .values(new_thread)
            .returning(Thread::as_returning())
            .get_result(connection)
            .await
    }
}
