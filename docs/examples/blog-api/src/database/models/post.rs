use diesel::dsl::{Eq, Filter, IsNotNull};
use serde::{Deserialize, Serialize};
use via::Result;

use crate::database::{
    models::user::{users, User},
    prelude::*,
};

pub use schema::posts;

// pub type All = Select<posts::table, posts::AllColumns>;
pub type Find = Filter<Public, Eq<posts::id, i32>>;
pub type Public = Filter<posts::table, IsNotNull<posts::published_at>>;

#[derive(Clone, Debug, Deserialize, AsChangeset)]
#[serde(rename_all = "camelCase")]
#[table_name = "posts"]
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
#[belongs_to(User)]
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

#[derive(Clone, Debug, Serialize)]
pub struct PostWithAuthor {
    #[serde(flatten)]
    post: Post,
    author: User,
}

fn published() -> IsNotNull<posts::published_at> {
    posts::published_at.is_not_null()
}

impl ChangeSet {
    pub async fn apply(self, pool: &Pool, id: i32) -> Result<Post> {
        let post = diesel::update(posts::table.filter(posts::id.eq(id)))
            .set(self)
            .returning(posts::all_columns)
            .get_result(&mut pool.get().await?)
            .await?;

        Ok(post)
    }
}

impl NewPost {
    pub async fn insert(self, pool: &Pool) -> Result<PostWithAuthor> {
        let author = User::find(pool, self.user_id).await?;
        let post = diesel::insert_into(posts::table)
            .values(self)
            .get_result(&mut pool.get().await?)
            .await?;

        Ok(PostWithAuthor { post, author })
    }
}

impl Post {
    pub async fn delete(pool: &Pool, id: i32) -> Result<()> {
        diesel::delete(posts::table.filter(posts::id.eq(id)))
            .execute(&mut pool.get().await?)
            .await?;

        Ok(())
    }

    pub async fn by_user(pool: &Pool, id: i32) -> Result<Vec<PostWithAuthor>> {
        Ok(posts::table
            .inner_join(users::table)
            .filter(posts::user_id.eq(id))
            .filter(published())
            .load(&mut pool.get().await?)
            .await?
            .into_iter()
            .map(|(post, author)| PostWithAuthor { post, author })
            .collect())
    }

    pub async fn find(pool: &Pool, id: i32) -> Result<PostWithAuthor> {
        let (post, author) = posts::table
            .inner_join(users::table)
            .filter(posts::id.eq(id))
            .filter(published())
            .first(&mut pool.get().await?)
            .await?;

        Ok(PostWithAuthor { post, author })
    }

    pub async fn public(pool: &Pool) -> Result<Vec<PostWithAuthor>> {
        Ok(posts::table
            .inner_join(users::table)
            .filter(posts::published_at.is_not_null())
            .load(&mut pool.get().await?)
            .await?
            .into_iter()
            .map(|(post, author)| PostWithAuthor { post, author })
            .collect())
    }
}
