mod models;
mod services;

use services::ApiService;
use via::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::new();

    app.service(ApiService);
    app.listen(("0.0.0.0", 8080)).await
}
