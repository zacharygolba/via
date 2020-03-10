mod models;
mod services;

use services::ArticleService;
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(ArticleService::new());
    app.listen().await
}
