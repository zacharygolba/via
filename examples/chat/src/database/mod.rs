pub mod query;
pub mod schema;

use bb8::{ManageConnection, Pool, PooledConnection, RunError};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

use crate::util::require_env;

pub type Connection<'a> = PooledConnection<'a, ConnectionManager>;
pub type ConnectionError = RunError<<ConnectionManager as ManageConnection>::Error>;
pub type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;

pub async fn establish_connection() -> Pool<ConnectionManager> {
    let database_url = require_env("DATABASE_URL");
    let manager = ConnectionManager::new(&database_url);
    let result = Pool::builder().build(manager).await;

    result.unwrap_or_else(|error| {
        panic!(
            "failed to establish database connection: url = {}, error = {}",
            database_url, error
        );
    })
}
