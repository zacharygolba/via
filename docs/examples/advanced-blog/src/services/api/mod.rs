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
    includes! {
        error::handler,
        middleware::cors(|allow| {
            allow.origin("*");
        }),
    }

    mount! {
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
        response::json(&self).respond()
    }
}
