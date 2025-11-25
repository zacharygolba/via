/// Generate filters scopes from binary expressions that work with the diesel
/// query dsl.
///
/// Scopes should correspond to an index in the database.
///
macro_rules! filters {
    (#[output] ==, $lhs:ty, $rhs:ty) => { diesel::dsl::Eq<$lhs, $rhs> };
    (#[expr] ==, $lhs:expr, $rhs:expr) => { $lhs.eq($rhs) };

    (#[output] !=, $lhs:ty, $rhs:ty) => { diesel::dsl::Ne<$lhs, $rhs> };
    (#[expr] !=, $lhs:expr, $rhs:expr) => { $lhs.ne($rhs) };

    (#[output] >,  $lhs:ty,   $rhs:ty) => { diesel::dsl::Gt<$lhs, $rhs> };
    (#[expr] >,   $lhs:expr, $rhs:expr) => { $lhs.gt($rhs) };

    (#[output] >=, $lhs:ty,   $rhs:ty) => { diesel::dsl::GtEq<$lhs, $rhs> };
    (#[expr] >=,  $lhs:expr, $rhs:expr) => { $lhs.ge($rhs) };

    (#[output] <,  $lhs:ty,   $rhs:ty) => { diesel::dsl::Lt<$lhs, $rhs> };
    (#[expr] <,   $lhs:expr, $rhs:expr) => { $lhs.lt($rhs) };

    (#[output] <=, $lhs:ty,   $rhs:ty) => { diesel::dsl::LtEq<$lhs, $rhs> };
    (#[expr] <=,  $lhs:expr, $rhs:expr) => { $lhs.le($rhs) };

    ($($vis:vis fn $by:ident($column:ident $op:tt $ty:ty) on $table:ident);+ $(;)?) => {
        $($vis fn $by(value: $ty) -> filters!(#[output] $op, $table::$column, $ty) {
            filters!(#[expr] $op, $table::$column, value)
        })+
    };
}

/// Generate sort scopes from a tuple of columns that can be used to order
/// results returned from the diesel query dsl.
///
/// Scopes should correspond to an index in the database.
///
/// For each scope defined, we generate a function that returns the tuple of
/// sort expressions and a module with the same name as the function. The
/// module exports a `columns` constant that can be used to reference the
/// columns returned from the sort scope when grouping results or filtering
/// before or after a keyset.
///
macro_rules! sorts {
    (#[output(desc)] $column:ty) => { diesel::dsl::Desc<$column> };
    (#[expr(desc)] $column:expr) => { $column.desc() };

    (#[output($(asc)?)] $column:ty) => { $column };
    (#[expr($(asc)?)] $column:expr) => { $column };

    (
        $($vis:vis fn $by:ident($($(#[$direction:tt])? $column:ident),+) on $table:ident);+
        $(;)?
    ) => {
        $(
            $vis fn $by() -> ($(sorts!(#[output($($direction)?)] $table::$column)),+) {
                ($(sorts!(#[expr($($direction)?)] $table::$column)),+)
            }

            $vis mod $by {
                use super::$table::{$($column),+};
                #[allow(dead_code, non_upper_case_globals)]
                pub const columns: ($($column),+) = ($($column),+);
            }
        )+
    };
}

pub mod message;
pub mod reaction;
pub mod subscription;
pub mod thread;
pub mod user;

pub use message::Message;
pub use thread::Thread;
pub use user::User;
