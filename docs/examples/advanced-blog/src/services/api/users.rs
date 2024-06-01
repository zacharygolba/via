use via::prelude::*;

use super::Document;
use crate::database::{
    models::{post::Post, user::*},
    Pool,
};

pub struct UserService {
    pool: Pool,
}

pub struct UsersService {
    pool: Pool,
}

impl UserService {
    pub fn new(pool: &Pool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[service("/:id")]
impl UserService {
    #[endpoint(GET, "/")]
    async fn show(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: User::find(&self.pool, id).await?,
        })
    }

    #[endpoint(PATCH, "/")]
    async fn update(&self, id: i32, mut context: Context) -> Result<impl Respond> {
        let _body: Document<ChangeSet> = context.read().json().await?;
        Ok(format!("Update User: {}", id))
    }

    #[endpoint(DELETE, "/")]
    async fn destroy(&self, id: i32) -> Result<impl Respond> {
        Ok(format!("Destroy User: {}", id))
    }
}

impl UsersService {
    pub fn new(pool: &Pool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[service("/users")]
impl UsersService {
    delegate! {
        UserService::new(&self.pool),
    }

    #[endpoint(GET, "/")]
    async fn index(&self) -> Result<impl Respond> {
        Ok(Document {
            data: User::all(&self.pool).await?,
        })
    }

    #[endpoint(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let body: Document<NewUser> = context.read().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[endpoint(GET, "/:id/posts")]
    async fn posts(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: Post::by_user(&self.pool, id).await?,
        })
    }
}
