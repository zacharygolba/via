use via::system::*;

use super::Document;
use crate::database::models::post::*;

connect!(PostsService);

#[service("/posts")]
impl PostsService {
    #[action(GET, "/")]
    async fn index(&self) -> Result<impl Respond> {
        Ok(Document {
            data: Post::public(&self.pool).await?,
        })
    }

    #[action(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let body: Document<NewPost> = context.read().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[action(GET, "/:id")]
    async fn show(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: Post::find(&self.pool, id).await?,
        })
    }

    #[action(PATCH, "/:id")]
    async fn update(&self, id: i32, mut context: Context) -> Result<impl Respond> {
        Ok(format!("Update Post: {}", id))
    }

    #[action(DELETE, "/:id")]
    async fn destroy(&self, id: i32) -> Result<impl Respond> {
        Ok(format!("Destroy Post: {}", id))
    }
}
