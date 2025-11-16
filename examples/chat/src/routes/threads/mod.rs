pub mod messages;
pub mod reactions;
pub mod subscriptions;

pub use authorization::authorization;

mod authorization;

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel_async::AsyncConnection;
use via::{Payload, Response};

use self::authorization::{Ability, Subscriber};
use crate::models::subscription::*;
use crate::models::thread::ThreadIncludes;
use crate::models::{Message, Subscription, Thread};
use crate::util::{DebugQueryDsl, PageAndLimit, Paginate, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Preconditions
    let current_user_id = *request.user()?;

    // Get pagination params from the URI query.
    let page = request.envelope().query::<PageAndLimit>()?;

    // Acquire a database connection and execute the query.
    let threads = Subscription::threads()
        .select(Thread::as_select())
        .filter(by_user(&current_user_id))
        .order(created_at_desc())
        .paginate(page)
        .debug_load(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&threads)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let current_user_id = *request.user()?;

    // Deserialize the request body into thread params.
    let (body, state) = request.into_future();
    let new_thread = body.await?.json()?;

    let thread = {
        let mut connection = state.pool().get_owned().await?;
        let future = connection.transaction(|trx| {
            Box::pin(async move {
                // Insert the thread into the threads table.
                let thread = Thread::create(new_thread)
                    .returning(Thread::as_returning())
                    .debug_result(trx)
                    .await?;

                // The owner of the thread has all auth claims.
                let association = Subscription::create(NewSubscription {
                    claims: AuthClaims::all(),
                    user_id: current_user_id,
                    thread_id: Some(thread.id),
                });

                // Associate the current user to the thread as an admin.
                association.debug_execute(trx).await?;

                Ok::<_, DieselError>(thread)
            })
        });

        future.await?
    };

    Response::build().status(201).json(&thread)
}

pub async fn show(request: Request, _: Next) -> via::Result {
    // Clone the thread that we eagerly loaded during authorization.
    let thread = request.subscription()?.thread().clone();

    // Acquire a database connection.
    let mut connection = request.state().pool().get_owned().await?;

    // Load the 25 most recent chat messages in the thread.
    let messages = Message::in_thread(&thread.id)
        .order(Message::created_at_desc())
        .limit(25)
        .debug_load(&mut connection)
        .await?;

    // Load the first 10 user subscriptions to the thread.
    let subscriptions = Subscription::users()
        .select(UserSubscription::as_select())
        .filter(by_thread(&thread.id))
        .order(created_at_desc())
        .limit(10)
        .debug_load(&mut connection)
        .await?;

    Response::build().json(&ThreadIncludes {
        thread,
        messages,
        subscriptions,
    })
}

pub async fn update(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?.thread_id();

    // Deserialize the request body into a thread change set.
    let (body, state) = request.into_future();
    let changes = body.await?.json()?;

    // Acquire a database connection and update the thread.
    let thread = Thread::update(&id, changes)
        .returning(Thread::as_returning())
        .debug_result(&mut state.pool().get().await?)
        .await?;

    Response::build().json(&thread)
}

pub async fn destroy(request: Request, _: Next) -> via::Result {
    let id = request.can(AuthClaims::MODERATE)?.thread_id();

    // Acquire a database connection.
    let mut connection = request.state().pool().get().await?;

    // Acquire a database connection and delete the thread.
    let rows = Thread::delete(&id).debug_execute(&mut connection).await?;

    // In debug builds assert that the number of affected rows is not zero.
    debug_assert!(rows > 0, "failed to delete thread: {}", &id);

    Response::build().status(204).finish()
}
