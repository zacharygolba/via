use crate::models::{article::*, Store, Uuid};
use serde::Serialize;
use via::prelude::*;

#[derive(Default)]
pub struct UserService;

#[service("/users")]
impl UserService {
    pub fn new() -> UserService {
        Default::default()
    }

    #[http(GET, "/")]
    async fn index(&self) -> impl Respond {}

    #[http(POST, "/")]
    async fn create(&self, mut context: Context) -> Result<impl Respond> {
        Ok(())
    }

    #[http(GET, "/:id")]
    async fn show(&self, id: Uuid) -> impl Respond {}

    #[http(PATCH, "/:id")]
    async fn update(&self, id: Uuid, mut context: Context) -> Result<impl Respond> {
        Ok(())
    }

    #[http(DELETE, "/:id")]
    async fn destroy(&self, id: Uuid) -> impl Respond {}
}
