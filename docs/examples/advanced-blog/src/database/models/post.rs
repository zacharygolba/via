use crate::database::prelude::*;
use diesel::dsl::{Eq, Filter, IsNotNull, Select};
use serde::{Deserialize, Serialize};
use via::prelude::*;

pub use schema::posts;

// pub type All = Select<posts::table, posts::AllColumns>;
pub type Find = Filter<Public, Eq<posts::id, i32>>;
pub type Public = Filter<posts::table, IsNotNull<posts::published_at>>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
    pub body: Option<String>,
    pub title: Option<String>,
    pub published_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "posts"]
pub struct NewPost {
    pub body: String,
    pub title: String,
    pub user_id: i32,
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: i32,
    pub body: String,
    pub title: String,
    #[serde(skip)]
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub published_at: Option<NaiveDateTime>,
}

fn published() -> IsNotNull<posts::published_at> {
    posts::published_at.is_not_null()
}

impl NewPost {
    pub async fn insert(self, pool: &Pool) -> Result<Post> {
        let insert = diesel::insert_into(posts::table);
        Ok(insert.values(self).get_result_async(pool).await?)
    }
}

impl Post {
    pub async fn by_user(pool: &Pool, id: i32) -> Result<Vec<Post>> {
        Ok(posts::table
            .filter(posts::user_id.eq(id))
            .filter(published())
            .load_async(pool)
            .await?)
    }

    pub async fn find(pool: &Pool, id: i32) -> Result<Post> {
        Ok(posts::table
            .filter(posts::id.eq(id))
            .filter(published())
            .first_async(pool)
            .await?)
    }

    pub async fn public(pool: &Pool) -> Result<Vec<Post>> {
        Ok(posts::table
            .filter(posts::published_at.is_not_null())
            .load_async(pool)
            .await?)
    }
}
