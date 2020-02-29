mod article;
mod store;

use article::{ArticleService, ArticleStore};
use via::prelude::*;

fn main() -> Result<(), Error> {
    let mut app = via::new();

    app.at("/articles").mount(ArticleService);
    app.inject(ArticleStore::default());
    app.listen()
}
