#[macro_use]
extern crate diesel;

mod database;
mod services;

use services::ApiService;
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();
    let pool = database::pool()?;

    app.service(ApiService::new(&pool));
    app.listen(("0.0.0.0", 8080)).await
}
