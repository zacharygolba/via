mod channel;
mod error;
mod upgrade;

pub use channel::{Channel, CloseCode, Message};
pub use error::{Result, Retry};
pub use upgrade::{Request, Upgrade};

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::ws::{self, Channel, Message};
/// use via::{App, Error, Payload};
///
/// async fn echo(mut channel: Channel, _: ws::Request) -> ws::Result {
///     loop {
///         let Some(message) = channel.next().await else {
///             break Ok(());
///         };
///
///         if let Message::Close(close) = &message {
///             close.as_ref().inspect(|(code, reason)| {
///                 eprintln!("{:?}: {:?}", code, reason);
///             });
///
///             break Ok(());
///         }
///
///         channel.send(message).await?;
///     }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Error> {
///     let mut app = App::new(());
///
///     // GET /echo ~> web socket upgrade.
///     app.route("/echo").respond(via::ws(echo));
///
///     Ok(())
/// }
///```
///
pub fn ws<State, F, R>(upgraded: F) -> Upgrade<F>
where
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result> + Send,
{
    Upgrade::new(upgraded)
}
