use via::prelude::*;

use super::Document;
use crate::database::{models::post::*, Pool};

pub struct PostsService {
    pool: Pool,
}

async fn authenticate(context: Context, next: Next) -> Result<impl IntoResponse> {
    println!("authenticate");
    next.call(context).await
}

impl PostsService {
    pub fn new(pool: &Pool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[service("/posts")]
impl PostsService {
    includes! {
        via::only![DELETE, PATCH, POST, PUT](authenticate),
    }

    #[endpoint(GET, "/")]
    async fn index(&self) -> Result<impl IntoResponse> {
        Ok(Document {
            data: Post::public(&self.pool).await?,
        })
    }

    #[endpoint(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl IntoResponse> {
        let body: Document<NewPost> = context.read().json().await?;

        Ok(Document {
            data: body.data.insert(&self.pool).await?,
        })
    }

    #[endpoint(GET, "/:id")]
    async fn show(&self, id: i32) -> Result<impl IntoResponse> {
        Ok(Document {
            data: Post::find(&self.pool, id).await?,
        })
    }

    #[endpoint(PATCH, "/:id")]
    async fn update(&self, id: i32, context: Context) -> Result<impl IntoResponse> {
        Ok(format!("Update Post: {}", id))
    }

    #[endpoint(DELETE, "/:id")]
    async fn destroy(&self, id: i32) -> Result<impl IntoResponse> {
        Ok(format!("Destroy Post: {}", id))
    }
}
