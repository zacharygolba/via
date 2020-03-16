use via::prelude::*;

#[derive(Clone, Copy)]
struct PostService;

#[derive(Clone, Copy)]
pub struct PostsService;

#[service("/:id")]
impl PostService {
    #[http(GET, "/")]
    async fn show(&self, id: String) -> impl Respond {
        format!("Show Post: {}", id)
    }

    #[http(PATCH, "/")]
    async fn update(&self, id: String) -> impl Respond {
        format!("Update Post: {}", id)
    }

    #[http(DELETE, "/")]
    async fn destroy(&self, id: String) -> impl Respond {
        format!("Destroy Post: {}", id)
    }
}

#[service("/posts")]
impl PostsService {
    services! {
        PostService,
    }

    #[http(GET, "/")]
    async fn index(&self) -> impl Respond {
        "List Posts"
    }

    #[http(POST, "/")]
    async fn create(&self, context: Context) -> impl Respond {
        "Create Post"
    }
}
