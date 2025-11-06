pub mod message;
pub mod reaction;
pub mod thread;
pub mod user;

use bb8::PooledConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

pub type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
pub type Connection<'a> = PooledConnection<'a, ConnectionManager>;
