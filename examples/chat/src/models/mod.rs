pub mod message;
pub mod reaction;
pub mod thread;
pub mod user;

use bb8::PooledConnection;
use diesel::pg::Pg;
use diesel::sql_types::Bool;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

pub type BoxFilter<T> = Box<dyn diesel::BoxableExpression<T, Pg, SqlType = Bool>>;

pub type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
pub type Connection<'a> = PooledConnection<'a, ConnectionManager>;
