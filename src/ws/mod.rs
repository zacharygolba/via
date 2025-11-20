mod error;
mod message;
mod upgrade;

pub use error::{Result, ResultExt};
pub use message::{Channel, CloseCode, Message};
pub use upgrade::{Request, Upgrade};

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::ws::{self, Channel, Message, Request};
/// use via::{App, Error, Payload};
///
/// async fn echo(mut channel: Channel, _: Request) -> ws::Result {
///     loop {
///         let Some(message) = channel.recv().await else {
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
///     app.route("/echo").to(ws::upgrade(echo));
///
///     Ok(())
/// }
///```
///
pub fn upgrade<State, F, R>(upgraded: F) -> Upgrade<F>
where
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result> + Send,
{
    Upgrade::new(upgraded)
}
