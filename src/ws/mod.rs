mod channel;
mod upgrade;

use crate::error::Error;

pub use channel::{Channel, CloseCode, Message};
pub use upgrade::{Context, Upgrade};

/// Upgrade the connection to a web socket.
///
/// # Example
///
/// ```
/// use via::ws::{self, Message};
/// use via::{App, Error, Payload};
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
///
/// async fn echo(mut channel: ws::Channel, _: ws::Context) -> via::Result<()> {
///     while let Some(message) = channel.next().await {
///         match message {
///             echo @ (Message::Binary(_) | Message::Text(_)) => {
///                 channel.send(echo).await?;
///             }
///             Message::Close(close) => {
///                 if let Some((code, reason)) = close {
///                     eprintln!("close: code = {}, reason = {:?}", u16::from(code), reason);
///                 }
///                 break;
///             }
///             _ => {}
///         }
///     }
///
///     Ok(())
/// }
///```
///
pub fn ws<State, F, R>(upgraded: F) -> Upgrade<F>
where
    F: Fn(Channel, Context<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<(), Error>> + Send,
{
    Upgrade::new(upgraded)
}
