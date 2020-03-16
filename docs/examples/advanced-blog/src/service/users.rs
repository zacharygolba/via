use via::prelude::*;

#[derive(Clone, Copy)]
struct UserService;

#[derive(Clone, Copy)]
pub struct UsersService;

#[service("/:id")]
impl UserService {
    #[http(GET, "/")]
    async fn show(&self, id: String) -> impl Respond {
        format!("Show User: {}", id)
    }

    #[http(PATCH, "/")]
    async fn update(&self, id: String) -> impl Respond {
        format!("Update User: {}", id)
    }

    #[http(DELETE, "/")]
    async fn destroy(&self, id: String) -> impl Respond {
        format!("Destroy User: {}", id)
    }
}

#[service("/users")]
impl UsersService {
    services! {
        UserService,
    }

    #[http(GET, "/")]
    async fn index(&self) -> impl Respond {
        "List Users"
    }

    #[http(POST, "/")]
    async fn create(&self, context: Context) -> impl Respond {
        "Create Users"
    }
}
