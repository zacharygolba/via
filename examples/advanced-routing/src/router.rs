use crate::service::{PostService, UserService};
use via::prelude::*;

pub struct ApiRouter;

pub struct RootRouter;

#[router]
impl ApiRouter {
    middleware! {
        helpers::cors(|allow| {
            allow.origin("*");
        }),
    }

    mount! {
        "/posts" => PostService,
        "/users" => UserService,
    }
}

#[router]
impl RootRouter {
    mount! {
        "/api" => Api,
    }
}
