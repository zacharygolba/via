use via::prelude::*;

use super::Document;
use crate::database::models::post::*;

connect!(PostsService);

#[service("/posts")]
impl PostsService {
    #[http(GET, "/")]
    async fn index(&self) -> impl Respond {
        Ok(Document {
            data: Post::public(&self.pool).await?,
        })
    }

    #[http(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let body: Document<NewPost> = context.body().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[http(GET, "/:id")]
    async fn show(&self, id: i32) -> Result<impl Respond> {
        Ok(Document {
            data: Post::find(&self.pool, id).await?,
        })
    }

    #[http(PATCH, "/:id")]
    async fn update(&self, id: i32, mut context: Context) -> Result<impl Respond> {
        Ok(format!("Update Post: {}", id))
    }

    #[http(DELETE, "/:id")]
    async fn destroy(&self, id: i32) -> Result<impl Respond> {
        Ok(format!("Destroy Post: {}", id))
    }
}
