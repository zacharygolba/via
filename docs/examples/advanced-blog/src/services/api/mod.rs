mod error;
mod posts;
mod users;

use self::{posts::PostsService, users::UsersService};
use serde::{Deserialize, Serialize};
use via::prelude::*;

connect!(ApiService);

#[derive(Debug, Deserialize, Serialize)]
pub struct Document<T> {
    pub data: T,
}

#[service("/api")]
impl ApiService {
    middleware! {
        error::handler,
        plugin::cors(|allow| {
            allow.origin("*");
        }),
    }

    services! {
        PostsService::new(&self.pool),
        UsersService::new(&self.pool),
    }
}

impl<T> Document<T> {
    pub fn new(data: T) -> Document<T> {
        Document { data }
    }
}

impl<T: Serialize> Respond for Document<T> {
    fn respond(self) -> Result<Response> {
        respond::json(&self).respond()
    }
}
