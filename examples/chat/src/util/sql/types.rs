use diesel::dsl::{self, Filter, Values};
use diesel::query_builder::{self, DeleteStatement, IntoUpdateTarget};

use crate::util::Id;

pub type Insert<T, New> = Values<query_builder::IncompleteInsertStatement<T>, New>;
pub type Update<'a, T, Pk, Changes> = dsl::Update<Filter<T, ById<'a, Pk>>, Changes>;
pub type Delete<'a, T, Pk> =
    DeleteStatement<T, <Filter<T, ById<'a, Pk>> as IntoUpdateTarget>::WhereClause>;

pub type ById<'a, Lhs> = dsl::Eq<Lhs, &'a Id>;
