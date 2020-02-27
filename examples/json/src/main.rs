mod article;
mod store;

use article::ArticleStore;
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.state(ArticleStore::default());
    app.at("/articles").scope(|mut articles: Location| {
        articles.route(article::create);
        articles.route(article::index);
        articles.route(article::find);
    });

    via::start(app).await
}
