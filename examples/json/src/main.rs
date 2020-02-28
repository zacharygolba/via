mod article;
mod store;

use article::{ArticleService, ArticleStore};
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.state(ArticleStore::default());
    app.at("/articles").scope(ArticleService);

    via::start(app).await
}
