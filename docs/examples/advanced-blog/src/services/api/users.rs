use via::prelude::*;

use super::Document;
use crate::database::models::{post::Post, user::*};

connect!(UserService);
connect!(UsersService);

#[service("/:id")]
impl UserService {
    #[http(GET, "/")]
    async fn show(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: User::find(&self.pool, id).await?,
        })
    }

    #[http(PATCH, "/")]
    async fn update(&self, id: i32, mut context: Context) -> Result<impl Respond> {
        let body: Document<ChangeSet> = context.body().json().await?;
        Ok(format!("Update User: {}", id))
    }

    #[http(DELETE, "/")]
    async fn destroy(&self, id: i32) -> Result<impl Respond> {
        Ok(format!("Destroy User: {}", id))
    }
}

#[service("/users")]
impl UsersService {
    services! {
        UserService::new(&self.pool),
    }

    #[http(GET, "/")]
    async fn index(&self) -> Result<impl Respond> {
        Ok(Document {
            data: User::all(&self.pool).await?,
        })
    }

    #[http(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let body: Document<NewUser> = context.body().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[http(GET, "/:id/posts")]
    async fn posts(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: Post::by_user(&self.pool, id).await?,
        })
    }
}
