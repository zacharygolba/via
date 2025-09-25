pub mod models;
pub mod schema;

pub mod prelude {
    pub use super::{Pool, schema};
    pub use chrono::NaiveDateTime;
    pub use diesel::prelude::*;
    pub use diesel_async::RunQueryDsl;
}

use diesel_async::{AsyncPgConnection, pooled_connection::AsyncDieselConnectionManager};
use std::env;
use via::BoxError;

type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
pub type Pool = bb8::Pool<ConnectionManager>;

pub async fn pool() -> Result<Pool, BoxError> {
    let config = ConnectionManager::new(env::var("DATABASE_URL")?);
    Ok(Pool::builder().build(config).await?)
}
