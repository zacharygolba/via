use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel_async::AsyncConnection;
use via::{Payload, Response};

use crate::models::channel::{Channel, NewChannel};
use crate::models::subscription::{self, NewSubscription};
use crate::schema::{channels, subscriptions};
use crate::util::paginate::{Page, Paginate};
use crate::util::{DebugQueryDsl, Session};
use crate::{Next, Request};

pub async fn index(request: Request, _: Next) -> via::Result {
    // Get pagination params from the URI query.
    let page = request.envelope().query::<Page>()?;

    // Acquire a database connection and execute the query.
    let channels = subscriptions::table
        .inner_join(channels::table)
        .select(Channel::as_select())
        .filter(subscription::by_user(request.user()?))
        .order(subscription::recent())
        .paginate(page)
        .debug_load(&mut request.app().database().await?)
        .await?;

    Response::build().json(&channels)
}

pub async fn create(request: Request, _: Next) -> via::Result {
    let user_id = request.user().copied()?;
    let (body, app) = request.into_future();

    // Deserialize a new channel from the request body.
    let new_channel = body.await?.json::<NewChannel>()?;
    let channel = {
        let mut connection = app.database().await?;
        let future = connection.transaction(|trx| {
            Box::pin(async move {
                // Insert the channel into the channels table.
                let channel = diesel::insert_into(channels::table)
                    .values(new_channel)
                    .returning(Channel::as_returning())
                    .debug_result(trx)
                    .await?;

                // Associate the current user to the channel as an admin.
                diesel::insert_into(subscriptions::table)
                    .values(NewSubscription::admin(user_id, *channel.id()))
                    .debug_execute(trx)
                    .await?;

                Ok::<_, DieselError>(channel)
            })
        });

        future.await?
    };

    Response::build().status(201).json(&channel)
}
