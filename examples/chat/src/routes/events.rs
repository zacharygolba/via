use crate::{Next, Request};

pub fn authorization(request: Request, next: Next) -> via::BoxFuture {
    next.call(request)
}

pub async fn index(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn create(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn show(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn update(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn destroy(_: Request, _: Next) -> via::Result {
    todo!()
}
