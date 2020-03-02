mod models;
mod services;

use services::ArticleService;
use via::prelude::*;

fn main() -> Result<()> {
    let mut app = App::new();

    app.mount(ArticleService::new());
    app.listen()
}
