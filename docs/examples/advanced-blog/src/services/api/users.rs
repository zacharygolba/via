use via::prelude::*;

use super::Document;
use crate::database::models::{post::Post, user::*};

connect!(UserService);
connect!(UsersService);

#[service("/:id")]
impl UserService {
    #[action(GET, "/")]
    async fn show(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: User::find(&self.pool, id).await?,
        })
    }

    #[action(PATCH, "/")]
    async fn update(&self, id: i32, mut context: Context) -> Result<impl Respond> {
        let body: Document<ChangeSet> = context.read().json().await?;
        Ok(format!("Update User: {}", id))
    }

    #[action(DELETE, "/")]
    async fn destroy(&self, id: i32) -> Result<impl Respond> {
        Ok(format!("Destroy User: {}", id))
    }
}

#[service("/users")]
impl UsersService {
    mount! {
        UserService::new(&self.pool),
    }

    #[action(GET, "/")]
    async fn index(&self) -> Result<impl Respond> {
        Ok(Document {
            data: User::all(&self.pool).await?,
        })
    }

    #[action(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let body: Document<NewUser> = context.read().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[action(GET, "/:id/posts")]
    async fn posts(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: Post::by_user(&self.pool, id).await?,
        })
    }
}
