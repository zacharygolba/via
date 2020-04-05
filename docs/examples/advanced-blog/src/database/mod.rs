pub mod models;
pub mod schema;

pub mod prelude {
    pub use super::{models, schema, Pool};
    pub use chrono::NaiveDateTime;
    pub use diesel::prelude::*;
    pub use tokio_diesel::{OptionalExtension, *};
}

use diesel::{prelude::*, r2d2};
use via::prelude::*;

type ConnectionManager = r2d2::ConnectionManager<PgConnection>;
pub type Pool = r2d2::Pool<ConnectionManager>;

pub fn pool() -> Result<Pool> {
    let url = dotenv::var("DATABASE_URL")?;
    Ok(Pool::new(ConnectionManager::new(url))?)
}
