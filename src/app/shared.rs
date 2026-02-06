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
/// original allocation created for the application at the time of its
/// construction.
///
/// For many ["safe" (read-only)](http::Method::is_safe) requests, you can
/// borrow the application as you would any other field of the request struct.
///
/// ### Example
///
/// ```
/// use std::sync::atomic::{AtomicU32, Ordering};
/// use via::{Next, Request, Response};
///
/// /// Our billion dollar application.
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
///     let place = request.app().visits.fetch_add(1, Ordering::Relaxed);
///     //                  ^^^
///     // App can be borrowed because request remains intact and we are not
///     // spawning async tasks that outlive the greet function.
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
/// ## Handling mutations
///
/// For non-idempotent HTTP requests (e.g., POST, PUT, DELETE), it is often
/// necessary to deconstruct the request in order to read the message body.
///
/// This can create a situation where the lifetime of the `Shared<App>` owned
/// by the request does not outlive the future returned by the service. However,
/// you may need to borrow the application to acquire a database connection or
/// persist mutations described in the request payload.
///
/// To solve this, ownership of the `Shared<App>` instance is transferred to
/// the caller via
/// [`Request::into_future`](crate::Request::into_future)
/// or
/// [`Request::into_parts`](crate::Request::into_parts).
/// This access pattern is safe, but clones of `Shared<App>` that escape the
/// future can introduce contention or leaks if not managed carefully.
///
/// ### Example
///
/// ```
/// use bb8::{ManageConnection, Pool};
/// use diesel::prelude::*;
/// use diesel_async::AsyncPgConnection;
/// use diesel_async::pooled_connection::AsyncDieselConnectionManager;
/// use http::StatusCode;
/// use via::request::Payload;
/// use via::{Next, Request, Response};
///
/// use models::{NewUser, User};
///
/// type ConnectionManager = AsyncDieselConnectionManager<AsyncPgConnection>;
///
/// /// Our billion dollar application.
/// struct Unicorn {
///     database: Pool<ConnectionManager>,
/// }
///
/// # mod models {
/// #     use chrono::{DateTime, Utc};
/// #     use diesel::prelude::*;
/// #     use serde::Deserialize;
/// #     use uuid::Uuid;
/// #
/// #     #[derive(Deserialize, Insertable)]
/// #     #[diesel(table_name = users)]
/// #     pub struct NewUser {
/// #         email: String,
/// #         username: String,
/// #     }
/// #
/// #     #[derive(Clone, Deserialize, Identifiable, Queryable, Selectable, Serialize)]
/// #     pub struct User {
/// #         id: Uuid,
/// #         email: String,
/// #         username: String,
/// #         created_at: DateTime<Utc>,
/// #         updated_at: DateTime<Utc>,
/// #     }
/// # }
/// #
/// async fn create_user(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     let (future, app) = request.into_future();
///     //           ^^^
///     // We are given ownership of the app owned by request so we can reference
///     // it after the future containing the JSON payload is ready.
///     //
///     // This is fine so long as app is dropped when this function returns.
///     //
///     let new_user = future.await?.json::<NewUser>()?;
///     let user = diesel::insert_into(users::table)
///         .values(new_user)
///         .returning(User::as_returning())
///         .debug_result(&mut app.database.get().await?)
///         .await?;
///
///     Response::build().status(StatusCode::CREATED).json(&user)
/// }
/// ```
///
/// If a middleware function spawns an async task that introduces a lifetime
/// different than that of the request, you must clone the Shared wrapper in
/// order to prevent a dangling borrow.
///
/// ### Example
///
/// ```
/// use tokio::io::{self, Sink};
/// use tokio::sync::Mutex;
/// use via::{Next, Request, Response};
///
/// /// An imaginary analytics service.
/// struct Telemetry(Mutex<Sink>);
///
/// /// Our billion dollar application.
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
///     // Spawn a detached async task that owns all of it's dependencies.
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
