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

#[via::service]
impl ArticleService {
    pub fn new() -> ArticleService {
        Default::default()
    }

    middleware! {
        helpers::cors(|allow| {
            allow.origin("*");
        }),
    }

    #[via::http("GET /")]
    async fn index(&self) -> impl Respond {
        let store = self.store.read().await;

        respond::json(&Document {
            data: store.all(),
        })
    }

    #[via::http("POST /")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        let NewArticle { body, title } = context.body().json().await?;
        let mut store = self.store.write().await;

        Ok(respond::json(&Document {
            data: store.insert(|id| Article { id, body, title }),
        }))
    }

    #[via::http("GET /:id")]
    async fn show(&self, id: Uuid) -> impl Respond {
        let store = self.store.read().await;
        let data = store.find(&id);

        respond::json(&Document { data }).status(match data {
            Some(_) => 200,
            None => 404,
        })
    }

    #[via::http("PATCH /:id")]
    async fn update(&self, id: Uuid, mut context: Context) -> Result<impl Respond> {
        let ChangeSet { body, title } = context.body().json().await?;
        let mut store = self.store.write().await;
        let data = store.update(&id, |article| {
            if let Some(value) = body {
                article.body = value;
            }

            if let Some(value) = title {
                article.title = value;
            }
        });

        Ok(respond::json(&Document { data }).status(match data {
            Some(_) => 200,
            None => 404,
        }))
    }

    #[via::http("DELETE /:id")]
    async fn destroy(&self, id: Uuid) -> impl Respond {
        let mut store = self.store.write().await;

        respond::status(match store.remove(&id) {
            Some(_) => 204,
            None => 404,
        })
    }
}