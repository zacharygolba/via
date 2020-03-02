use crate::models::{article::*, Store, Uuid};
use serde::Serialize;
use via::prelude::*;

#[derive(Default)]
pub struct ArticleService {
    store: Store<Article>,
}

#[derive(Debug, Serialize)]
struct Document<T: Serialize> {
    data: T,
}

#[service("/articles")]
impl ArticleService {
    pub fn new() -> ArticleService {
        Default::default()
    }

    middleware! {
        helpers::cors(|allow| {
            allow.origin("*");
        }),
    }

    #[expose(GET, "/")]
    async fn list(&self) -> impl Respond {
        let store = self.store.read().await;

        respond::json(&Document {
            data: store.all(),
        })
    }

    #[expose(GET, "/:id")]
    async fn find(&self, id: Uuid) -> impl Respond {
        let store = self.store.read().await;
        let data = store.find(&id);

        respond::json(&Document { data }).status(match data {
            Some(_) => 200,
            None => 404,
        })
    }

    #[expose(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let NewArticle { body, title } = context.body().json().await?;
        let mut store = self.store.write().await;

        Ok(respond::json(&Document {
            data: store.insert(|id| Article { id, body, title }),
        }))
    }
}
