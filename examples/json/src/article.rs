use crate::store::Store;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use via::{middleware, prelude::*};

pub type ArticleStore = Store<Article>;

pub struct ArticleService;

#[derive(Debug, Serialize)]
pub struct Article {
    id: Uuid,
    body: String,
    title: String,
}

#[derive(Debug, Deserialize)]
pub struct NewArticle {
    body: String,
    title: String,
}

#[via::router]
impl ArticleService {
    middleware! {
        helpers::cors(|allow| {
            allow.origin("*");
        }),
    }

    #[post("/")]
    async fn create(mut context: Context) -> Result<impl Respond, Error> {
        let NewArticle { body, title } = context.body().json().await?;
        let mut store = context.state::<ArticleStore>()?.write().await;

        Ok(json! {
            "article": store.insert(|id| Article { id, body, title }),
        })
    }

    #[get("/")]
    async fn index(context: Context) -> Result<impl Respond, Error> {
        let store = context.state::<ArticleStore>()?.read().await;

        Ok(json! {
            "articles": store.all(),
        })
    }

    #[get("/:id")]
    async fn find(id: Uuid, context: Context) -> Result<impl Respond, Error> {
        let store = context.state::<ArticleStore>()?.read().await;
        let article = store.find(&id);

        Ok(json! { "article": &article }.status(match article {
            Some(_) => 200,
            None => 404,
        }))
    }
}
