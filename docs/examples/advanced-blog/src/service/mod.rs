mod posts;
mod users;

use self::{posts::PostsService, users::UsersService};
use via::prelude::*;

pub struct ApiService;

#[service("/api")]
impl ApiService {
    middleware! {
        plugin::cors(|allow| {
            allow.origin("*");
        }),
    }

    services! {
        PostsService,
        UsersService,
    }
}
