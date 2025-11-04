use via::{Payload, Response};

use crate::models::thread::{NewThread, Thread};
use crate::util::Authenticate;
use crate::{Next, Request};

pub async fn index(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let (head, future) = request.into_future();
    let mut params = future.await?.serde_json::<NewThread>()?;

    params.owner_id = Some(head.current_user()?.id);

    let mut connection = head.state().pool().get().await?;
    let thread = Thread::create(&mut connection, params).await?;

    Response::build().status(201).json(&thread)
}

pub async fn authorization(request: Request, next: Next) -> via::Result {
    next.call(request).await
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

pub async fn add(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn remove(_: Request, _: Next) -> via::Result {
    todo!()
}
