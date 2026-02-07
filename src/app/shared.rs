use std::fmt::{self, Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

/// A thread-safe, reference-counting pointer to the application.
///
/// An application is a user-defined struct that bundles together singleton
/// resources whose lifetime matches that of the process in which it is created.
///
/// `Shared` wraps an application and provides per-request ownership of the
/// container. This allows resources to flow through async code without creating
/// dangling borrows or introducing implicit lifetimes.
///
/// Cloning a `Shared<App>` is inexpensive: it performs an atomic increment when
/// cloned and an atomic decrement when dropped. When a client request is
/// received, the `Shared` wrapper is cloned and ownership of the clone is
/// transferred to the request.
///
/// # Safe Access
///
/// Async functions are compiled into state machines that may be suspended across
/// `.await` points. Any borrow that outlives the data it references becomes
/// invalid when the future is resumed, violating Rust’s safety guarantees.
///
/// For many ["safe" (read-only)](http::Method::is_safe) requests, the application
/// can be borrowed directly from the request without cloning or taking ownership
/// of the underlying `Shared<App>`.
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
///
///     let place = request.app().visits.fetch_add(1, Ordering::Relaxed);
///     //                  ^^^
///     // The application may be borrowed here because the request remains
///     // intact for the duration of this function and no detached tasks
///     // are spawned.
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
/// necessary to consume the request in order to read the message body.
///
/// In these cases, ownership of the `Shared<App>` is returned to the caller.
/// This commonly occurs when a mutation requires acquiring a database
/// connection or persisting state derived from the request body.
///
/// This access pattern is safe, but any clone of `Shared<App>` that escapes the
/// request future extends the lifetime of the application container and should
/// be treated as an intentional design choice.
///
/// ### Example
///
/// ```
/// async fn create_user(request: Request<Unicorn>, _: Next<Unicorn>) -> via::Result {
///     let (future, app) = request.into_future();
///     //           ^^^
///     // Ownership of the application is transferred so it can be accessed
///     // after the request body future resolves.
///     //
///     // This is correct so long as `app` is dropped before the function
///     // returns.
///     let new_user = future.await?.json::<NewUser>()?;
///
///     let user = {
///         let mut connection = app.database.get().await?;
///         diesel::insert_into(users::table)
///             .values(new_user)
///             .returning(User::as_returning())
///             .get_result(&mut connection)
///             .await?
///     };
///
///     Response::build()
///         .status(StatusCode::CREATED)
///         .json(&user)
/// }
/// ```
///
/// See: [`request.into_future()`] and [`request.into_parts()`].
///
/// ## Detached Tasks and Atomic Contention
///
/// `Shared<App>` relies on an atomic reference count to track ownership across
/// threads. In typical request handling, the clone/drop rhythm closely follows
/// the request lifecycle. This predictable cadence helps keep **atomic
/// contention low**.
///
/// Contention can be understood as waves:
///
/// - Each request incrementing or decrementing the reference count forms a peak
/// - If all requests align perfectly, peaks add together, increasing contention
/// - In practice, requests are staggered in time, causing the peaks to partially
///   cancel and flatten
///
/// ```text
/// 'process: ──────────────────────────────────────────────────────────────────────────>
///            |                             |                              |
///        HTTP GET                          |                              |
///       app.clone()                        |                              |
///    incr strong_count                 HTTP GET                           |
///            |                        app.clone()                         |
///            |                     incr strong_count                  HTTP POST
///        List Users                        |                         app.clone()
/// ┌──────────────────────┐                 |                      incr strong_count
/// |   borrow req.app()   |        Web Socket Upgrade                      |
/// |  acquire connection  |      ┌─────────────────────┐                   |
/// |   respond with json  |      |     app.clone()     |              Create User
/// └──────────────────────┘      |   spawn async task  |─┐     ┌──────────────────────┐
///     decr strong_count         | switching protocols | |     |   req.into_future()  |
///            |                  └─────────────────────┘ |     |     database trx     |
///            |                     decr strong_count    |     |       respond        |
///            |                             |            |     └──────────────────────┘
///            |                             |            |        decr strong_count
///            |                             |            |                 |
///            |                             |            └─>┌──────────────┐
///            |                             |               |  web socket  |
///            |                             |               └──────────────┘
///            |                             |               decr strong_count
///            |                             |                              |
/// ┌──────────|─────────────────────────────|──────────────────────────────|───────────┐
/// | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | | |
/// └──────────|─────────────────────────────|──────────────────────────────|───────────┘
///            |                             |                              |
///       uncontended                   uncontended                     contended
/// ```
///
/// Detached tasks disrupt this rhythm:
///
/// - Their increments and decrements occur out of phase with the request
///   lifecycle
/// - This can temporarily spike contention and extend resource lifetimes beyond
///   the request
///
/// Keeping `Shared<App>` clones phase-aligned with the request lifecycle
/// minimizes atomic contention and keeps resource lifetimes predictable. When
/// the Arc is dropped deterministically as the middleware future resolves,
/// leaks and unintended retention become significantly easier to detect.
///
/// ### Example
///
/// ```
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
/// #### Timing and Side-Channel Awareness
///
/// Each clone and drop of `Shared<App>` performs an atomic operation. When these
/// operations occur out of phase with normal request handling (for example, in
/// detached background tasks), they can introduce observable timing differences.
///
/// In high-assurance systems, such differences may:
///
/// - Act as unintended signals to an attacker
/// - Reveal the presence of privileged handlers (e.g., [web socket upgrades])
/// - Correlate background activity with specific request types
///
/// In these cases, preserving a uniform request rhythm may be more valuable than
/// minimizing contention. These tradeoffs should be made deliberately and
/// documented, as they exchange throughput and modularity for reduced
/// observability.
///
/// [`request.into_future()`]: crate::Request::into_future
/// [`request.into_parts()`]: crate::Request::into_parts
/// [web socket upgrades]: ../src/via/ws/upgrade.rs.html#256-262
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
