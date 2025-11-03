use crate::{Next, Request};
use via::{Response, raise};

pub async fn index(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn create(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn show(request: Request, _: Next) -> via::Result {
    let id = request.param("thread-id").parse()?;
    let chat = request.state().as_ref();
    let future = chat.thread(&id, |thread| Response::build().json(&thread));

    future.await.unwrap_or_else(|| raise!(404))
}

pub async fn update(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn destroy(_: Request, _: Next) -> via::Result {
    todo!()
}
