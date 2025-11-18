// This macro is intended to be the starting point of a declarative way of
// specifying filter expressions that use or partially use an index on a table.
macro_rules! filters {
    (#[expr] $by:ident($column:ident == $ty:ty) on $table:ident) => {
        pub fn $by(value: $ty) -> diesel::dsl::Eq<$table::$column, $ty> {
            $table::$column.eq(value)
        }
    };

    ($($by:ident($($expr:tt)+) on $table:ident),+ $(,)?) => {
        $(filters! { #[expr] $by($($expr)+) on $table })+
    };
}

macro_rules! sorts {
    ($table:ident) => {
        pub fn by_recent() -> (diesel::dsl::Desc<$table::created_at>, $table::id) {
            ($table::created_at.desc(), $table::id)
        }

        pub mod by_recent {
            use super::$table::{created_at, id};

            #[allow(dead_code, non_upper_case_globals)]
            pub const columns: (created_at, id) = (created_at, id);
        }
    };
}

pub mod message;
pub mod reaction;
pub mod subscription;
pub mod thread;
pub mod user;

use bb8::PooledConnection;
pub use message::Message;
pub use thread::Thread;
pub use user::User;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

pub type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
pub type Connection<'a> = PooledConnection<'a, ConnectionManager>;
