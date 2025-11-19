use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel_async::AsyncConnection;
use via::{Payload, Response};

use crate::models::subscription::{self, AuthClaims, NewSubscription, Subscription, subscriptions};
use crate::models::thread::{NewThread, Thread, threads};
use crate::util::paginate::{Page, Paginate};
use crate::util::{DebugQueryDsl, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let page = request.envelope().query::<Page>()?;

    // Acquire a database connection and execute the query.
    let threads = Subscription::threads()
        .select(Thread::as_select())
        .filter(subscription::by_user(request.user()?))
        .order(subscription::recent())
        .paginate(page)
        .debug_load(&mut request.state().pool().get().await?)
        .await?;

    Response::build().json(&threads)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let user_id = request.user().cloned()?;

    // Deserialize the request body into thread params.
    let (body, state) = request.into_future();
    let new_thread = body.await?.json::<NewThread>()?;

    let thread = {
        let mut connection = state.pool().get().await?;
        let future = connection.transaction(|trx| {
            Box::pin(async move {
                // Insert the thread into the threads table.
                let thread = diesel::insert_into(threads::table)
                    .values(new_thread)
                    .returning(Thread::as_returning())
                    .debug_result(trx)
                    .await?;

                // Associate the current user to the thread as an admin.
                diesel::insert_into(subscriptions::table)
                    .values(NewSubscription {
                        user_id,
                        claims: AuthClaims::all(),
                        thread_id: Some(thread.id().clone()),
                    })
                    .debug_execute(trx)
                    .await?;

                Ok::<_, DieselError>(thread)
            })
        });

        future.await?
    };

    Response::build().status(201).json(&thread)
}
