use via::prelude::*;

pub struct PostService;

#[router]
impl PostService {
    #[get("/")]
    async fn index(context: Context) -> ! {
        todo!()
    }

    #[post("/")]
    async fn create(context: Context) -> ! {
        todo!()
    }

    #[get("/:id")]
    async fn find(id: u64, context: Context) -> ! {
        todo!()
    }

    #[patch("/:id")]
    async fn update(id: u64, context: Context) -> ! {
        todo!()
    }

    #[delete("/:id")]
    async fn destroy(id: u64, context: Context) -> ! {
        todo!()
    }
}
