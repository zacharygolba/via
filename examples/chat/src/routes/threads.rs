use diesel::pg::Pg;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use via::{Payload, Response};

use crate::models::message::Message;
use crate::models::thread::*;
use crate::util::{Authenticate, LimitAndOffset};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let LimitAndOffset(limit, offset) = request.envelope().query()?;

    // Build the query from URI params.
    let query = Thread::query()
        .order(created_at_desc())
        .limit(limit)
        .offset(offset);

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    // Acquire a database connection and execute the query.
    let threads: Vec<ThreadWithOwner> = {
        let pool = request.state().pool();
        let mut conn = pool.get().await?;

        query.load(&mut conn).await?
    };

    Response::build().json(&threads)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = request.envelope().current_user()?.id;

    // Deserialize the request body into message params.
    let (body, state) = request.into_future();
    let mut params = body.await?.json::<NewThread>()?;

    // Source foreign keys from request metadata when possible.
    params.owner_id = Some(current_user_id);

    // Build the insert statement with the params from the body.
    let insert = diesel::insert_into(Thread::TABLE)
        .values(params)
        .returning(Thread::as_returning());

    // Print the query to stdout in debug mode.
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&insert));
    }

    // Acquire a database connection and execute the insert.
    let thread = {
        let mut conn = state.pool().get().await?;
        insert.get_result(&mut conn).await?
    };

    Response::build().status(201).json(&thread)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Preconditions
    let id = request.envelope().param("thread-id").parse()?;

    // Acquire a database connection.
    let mut conn = request.state().pool().get().await?;

    let query = Thread::query().filter(by_id(id));

    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    let (thread, owner) = query.first(&mut conn).await?;
    let query = Message::belonging_to(&thread)
        .select(Message::as_select())
        .limit(25);

    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<Pg, _>(&query));
    }

    let thread = ThreadDetails {
        messages: query.load(&mut conn).await?,
        thread,
        owner,
    };

    Response::build().json(&thread)
}

pub async fn update(_: Request, _: Next) -> via::Result {
    todo!()
}

pub async fn destroy(_: Request, _: Next) -> via::Result {
    todo!()
}
