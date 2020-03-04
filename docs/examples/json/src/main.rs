mod models;
mod services;

use services::ArticleService;
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();

    app.at("/articles").mount(ArticleService::new());
    app.listen().await
}
