use std::fmt::{self, Debug, Formatter};
use std::iter::Extend;

use super::{parse_query_params, ParamType, QueryParamValues};

pub struct Params {
    did_parse_query: bool,
    param_indices: Vec<ParamType>,
}

impl Params {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            did_parse_query: false,
            param_indices: Vec::with_capacity(capacity),
        }
    }

    pub fn did_parse_query(&self) -> bool {
        self.did_parse_query
    }

    pub fn get_path_param(&self, predicate: &str) -> Option<&(usize, usize)> {
        self.param_indices.iter().find_map(|kind| match kind {
            ParamType::Path(name, at) if *name == predicate => Some(at),
            _ => None,
        })
    }

    pub fn get_query_params<'a, 'b>(
        &'a mut self,
        query: &'a str,
        predicate: &'b str,
    ) -> QueryParamValues<'a, 'b> {
        if !self.did_parse_query {
            self.parse_query_params(query);
        }

        let params = self.param_indices.iter().filter_map(|kind| match kind {
            ParamType::Query(name, at) if *name == predicate => Some(at),
            _ => None,
        });

        QueryParamValues::new(predicate, query, params.collect())
    }

    pub fn parse_query_params(&mut self, query: &str) {
        self.param_indices
            .extend(parse_query_params(query).map(|parts| parts.into()));

        self.did_parse_query = true;
    }
}

impl Debug for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.param_indices, f)
    }
}

impl Extend<(&'static str, (usize, usize))> for Params {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (&'static str, (usize, usize))>,
    {
        self.param_indices
            .extend(iter.into_iter().map(|parts| parts.into()));
    }
}
