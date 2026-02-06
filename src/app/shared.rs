use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

/// A thread-safe, reference-counting pointer to the application.
///
/// An application is a user-defined struct that bundles together singleton
/// resources whose liveness matches that of the process in which it is
/// created.
///
/// `Shared` wraps your application and provides per-request ownership of the
/// container. This allows resources to be passed through async code without
/// creating dangling borrows or introducing implicit lifetimes.
///
/// Cloning a `Shared<App>` is inexpensive: it performs an atomic increment
/// when cloned and an atomic decrement when dropped.
///
/// # Safe Access
///
/// Async functions are transformed into state machines that may be suspended
/// across `.await` points. Any borrow that outlives the data it references
/// becomes invalid when the future is resumed, violating Rust’s safety
/// guarantees.
///
/// When a client request is received, the `Shared` wrapper around your
/// application is cloned by incrementing the strong reference count of the
/// original allocation created at application startup.
///
/// For many
/// ["safe" (read-only)](http::Method::is_safe)
/// requests, the application can be borrowed directly from the request for the
/// duration of the handler.
///
/// ### Example
///
/// ```
/// use std::sync::atomic::{AtomicU32, Ordering};
/// use via::{Next, Request, Response};
///
/// /// Our billion dollar application.
/// ///
/// struct Unicorn {
///     visits: AtomicU32,
/// }
///
/// fn inflect(place: u32) -> &'static str {
///     if (11..=13).contains(&place) {
///         return "th";
///     }
///
///     match place % 10 {
///         1 => "st",
///         2 => "nd",
///         3 => "rd",
///         _ => "th",
///     }
/// }
///
/// async fn greet(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     let name = request.envelope().param("name").decode().into_result()?;
///
///     let place = request.app().visits.fetch_add(1, Ordering::Relaxed);
///     //                  ^^^
///     // The application can be borrowed here because the request remains
///     // intact for the duration of this function and no detached async tasks
///     // are spawned.
///     //
///     let suffix = inflect(place);
///
///     Response::build().text(format!(
///         "Hello, {}! You are the {}{} visitor.",
///         name, place, suffix,
///     ))
/// }
/// ```
///
/// ## Handling Mutations
///
/// For non-idempotent HTTP requests (e.g., DELETE, PATCH, POST), it is often
/// necessary to deconstruct the request in order to read the message body.
///
/// In these cases, the `Shared<App>` owned by the request may need to outlive
/// the point at which the request is consumed. This commonly occurs when a
/// mutation requires acquiring a database connection or persisting state
/// derived from the request body.
///
/// To support this pattern, ownership of the `Shared<App>` instance can be
/// transferred to the caller via
/// [`Request::into_future`](crate::Request::into_future)
/// or
/// [`Request::into_parts`](crate::Request::into_parts).
///
/// This access pattern is safe, but any clone of `Shared<App>` that escapes
/// the request’s future extends the lifetime of the application container and
/// should be treated as an intentional design choice.
///
/// ### Example
///
/// ```
/// use bb8::{ManageConnection, Pool};
/// use diesel::prelude::*;
/// use diesel_async::{AsyncPgConnection, RunQueryDsl};
/// use diesel_async::pooled_connection::AsyncDieselConnectionManager;
/// use http::StatusCode;
/// use via::request::Payload;
/// use via::{Next, Request, Response};
///
/// use models::{users, NewUser, User};
///
/// type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
///
/// /// Our billion dollar application.
/// ///
/// struct Unicorn {
///     database: Pool<ConnectionManager>,
/// }
/// #
/// # mod models {
/// #     use diesel::prelude::*;
/// #     use serde::{Deserialize, Serialize};
/// #     use uuid::Uuid;
/// #
/// #     diesel::table! {
/// #        users (id) {
/// #            id -> Uuid,
/// #            email -> Text,
/// #            username -> Text,
/// #        }
/// #     }
/// #
/// #     #[derive(Deserialize, Insertable)]
/// #     #[diesel(table_name = users)]
/// #     pub struct NewUser {
/// #         email: String,
/// #         username: String,
/// #     }
/// #
/// #     #[derive(Clone, Queryable, Selectable, Serialize)]
/// #     pub struct User {
/// #         id: Uuid,
/// #         email: String,
/// #         username: String,
/// #     }
/// # }
/// #
///
/// async fn create_user(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     let (future, app) = request.into_future();
///     //           ^^^
///     // Ownership of the application is transferred so it can be accessed
///     // after the request body future resolves.
///     //
///     // This is correct so long as `app` is dropped when this function
///     // returns.
///     //
///     let new_user = future.await?.json::<NewUser>()?;
///
///     let user = diesel::insert_into(users::table)
///         .values(new_user)
///         .returning(User::as_returning())
///         .get_result(&mut app.database.get().await?)
///         .await?;
///
///     Response::build()
///         .status(StatusCode::CREATED)
///         .json(&user)
/// }
/// ```
///
/// ## Detached Tasks and Atomic Contention
///
/// `Shared<App>` relies on an atomic reference count to track ownership across
/// threads. In normal request handling, the clone/drop rhythm follows the
/// request lifecycle.
///
/// This predictable rhythm keeps **atomic contention low**. Think of
/// contention like **waves**:
///
/// - Each request incrementing or decrementing the Arc is a wave peak.
///
/// - If all requests hit at exactly the same moment, peaks **add up**,
///   creating contention.
///
/// - If requests are **staggered in time** (naturally spread by route latency
///   or concurrency limits), the waves **partially cancel**, flattening the
///   peaks.
///
/// ### Contention Over Time
///
/// ```text
/// 'process: ─────────────────────────────────────────────────────────────────────────>
///            |                             |                             |
///        HTTP GET                          |                             |
///       app.clone()                        |                             |
///    incr strong_count                 HTTP GET                          |
///            |                        app.clone()                    HTTP POST
///            |                     incr strong_count                app.clone()
///        List Users                        |                     incr strong_count
/// ┌──────────────────────┐                 |                             |
/// |   borrow req.app()   |        Web Socket Upgrade                     |
/// |  acquire connection  |      ┌─────────────────────┐             Create User
/// |   respond with json  |      |     app.clone()     |      ┌──────────────────────┐
/// └──────────────────────┘      |   spawn async task  |─┐    |   req.into_future()  |
///     decr strong_count         | switching protocols | |    |     database trx     |
///            |                  └─────────────────────┘ |    |       respond        |
///            |                     decr strong_count    |    └──────────────────────┘
///            |                             |            |        decr strong_count
///            |                             |            |                |
///            |                             |            └─>┌─────────────┐
///            |                             |               |  websocket  |
///            |                             |               └─────────────┘
///            |                             |              decr strong_count
///            |                             |                             |
/// ┌──────────|─────────────────────────────|─────────────────────────────|───────────┐
/// | | | | | | | | | | | | | | | | | | | | | | | | | | |  | | | | | | | | | | | | | | |
/// └──────────|─────────────────────────────|─────────────────────────────|───────────┘
///            |                             |                             |
///       uncontended                   uncontended                    contended
/// ```
///
/// Detached tasks break this rhythm:
///
/// - The Arc increment/decrement of the detached task is **out of phase** with
///   the main request waves.
/// - This can spike contention temporarily and extend the logical lifetime of
///   resources beyond the request.
///
/// **Guideline:**
///
/// - Detached tasks should be rare. Only clone the `Shared<App>` into a task
///   when necessary.
/// - Most requests should follow the normal rhythm to **keep contention low
///   and resource lifetimes predictable**.
///
/// ### Example
///
/// ```
/// use tokio::io::{self, Sink};
/// use tokio::sync::Mutex;
/// use via::{Next, Request, Response};
///
/// /// An imaginary analytics service.
/// ///
/// struct Telemetry(Mutex<Sink>);
///
/// /// Our billion dollar application.
/// ///
/// struct Unicorn {
///     telemetry: Telemetry,
/// }
///
/// impl Telemetry {
///     async fn report(&self, mut visitor: &[u8]) -> io::Result<()> {
///         let mut guard = self.0.lock().await;
///         io::copy(&mut visitor, &mut *guard).await?;
///         Ok(())
///     }
/// }
///
/// async fn greet(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     let name = request.envelope().param("name").decode().into_result()?;
///
///     // Spawn a detached task that explicitly owns all of its dependencies.
///     tokio::spawn({
///         let app = request.app().clone();
///         let name = name.clone().into_owned();
///         async move { app.telemetry.report(name.as_bytes()).await }
///     });
///
///     Response::build().text(format!("Hello, {}!", name))
/// }
/// ```
///
/// The guidance above favors keeping `Shared<App>` clones phase-aligned with
/// the request lifecycle in order to minimize atomic contention and keep
/// resource lifetimes predictable.
///
/// However, this is a *performance and observability* guideline — not a hard
/// safety rule.
///
/// In some contexts, intentionally *avoiding* a detached clone may be the
/// correct choice.
///
/// #### Timing and Side-Channel Awareness
///
/// Each clone and drop of `Shared<App>` performs an atomic operation. When
/// these operations occur out of phase with normal request handling (for
/// example, in detached background tasks), they can introduce observable
/// timing differences.
///
/// In high-assurance systems, such differences may:
///
/// - Act as unintended signals to an attacker
/// - Reveal the presence of privileged handlers (e.g., websocket upgrades)
/// - Correlate background activity with specific request types
///
/// In these cases, preserving a uniform request rhythm may be more valuable
/// than minimizing contention.
///
/// #### Intentional Deviations
///
/// You may choose to deviate from the standard pattern when:
///
/// - A detached task would introduce a distinctive timing profile
/// - Uniformity across request types is more important than peak throughput
/// - The background work can be structured to avoid accessing `App` entirely
///
/// Such decisions should be made deliberately and documented, as they trade
/// throughput and modularity for reduced observability.
///
pub struct Shared<App>(Arc<App>);

impl<App> Shared<App> {
    pub(super) fn new(value: App) -> Self {
        Self(Arc::new(value))
    }
}

impl<App> AsRef<App> for Shared<App> {
    #[inline]
    fn as_ref(&self) -> &App {
        &self.0
    }
}

impl<App> Clone for Shared<App> {
    #[inline]
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<App> Debug for Shared<App> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Shared").finish()
    }
}

impl<App> Deref for Shared<App> {
    type Target = App;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
