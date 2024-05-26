mod error;
mod posts;
mod users;

use serde::{Deserialize, Serialize};
use via::prelude::*;

use self::{posts::PostsService, users::UsersService};
use crate::database::Pool;

pub struct ApiService {
    pool: Pool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Document<T> {
    pub data: T,
}

impl ApiService {
    pub fn new(pool: &Pool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[service("/api")]
impl ApiService {
    delegate! {
        PostsService::new(&self.pool),
        UsersService::new(&self.pool),
    }

    includes! {
        error::handler,
    }
}

impl<T> Document<T> {
    pub fn new(data: T) -> Document<T> {
        Document { data }
    }
}

impl<T: Serialize> Respond for Document<T> {
    fn respond(self) -> Result<Response> {
        response::json(&self).respond()
    }
}
