mod article;
mod user;

use self::{article::*, user::*};
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
        ArticleService::new(),
        UserService::new(),
    }

    #[http("/*path")]
    async fn catch(&self, context: Context, next: Next) -> impl Respond {
        match next.call(context).await {
            Ok(response) => Ok(response),
            Err(error) => Err(error.json()),
        }
    }
}
