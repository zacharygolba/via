use crate::database::prelude::*;
use serde::{Deserialize, Serialize};
use via::Result;

pub use schema::users;

#[derive(Clone, Debug, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub username: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub username: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i32,
    pub username: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ChangeSet {
    pub async fn apply(self, pool: &Pool, id: i32) -> Result<User> {
        let post = diesel::update(users::table.filter(users::id.eq(id)))
            .set(self)
            .returning(users::all_columns)
            .get_result(&mut pool.get().await?)
            .await?;

        Ok(post)
    }
}

impl NewUser {
    pub async fn insert(self, pool: &Pool) -> Result<User> {
        let insert = diesel::insert_into(users::table);
        Ok(insert
            .values(self)
            .get_result(&mut pool.get().await?)
            .await?)
    }
}

impl User {
    pub async fn all(pool: &Pool) -> Result<Vec<User>> {
        Ok(users::table
            .select(users::all_columns)
            .load(&mut pool.get().await?)
            .await?)
    }

    pub async fn delete(pool: &Pool, id: i32) -> Result<()> {
        diesel::delete(users::table.filter(users::id.eq(id)))
            .execute(&mut pool.get().await?)
            .await?;

        Ok(())
    }

    pub async fn find(pool: &Pool, id: i32) -> Result<User> {
        Ok(users::table
            .filter(users::id.eq(id))
            .first(&mut pool.get().await?)
            .await?)
    }
}
