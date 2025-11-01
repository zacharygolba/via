mod channel;
mod upgrade;

use std::ops::ControlFlow;

use crate::Error;

pub use channel::{Channel, CloseCode, Message};
pub use upgrade::{Request, Upgrade};

pub type Result<T = ()> = std::result::Result<T, ControlFlow<Error, Error>>;

pub trait Retry {
    type Output;
    fn or_break(self) -> Result<Self::Output>;
    fn or_continue(self) -> Result<Self::Output>;
}

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
/// async fn echo(mut channel: ws::Channel, _: ws::Request) -> ws::Result {
///     use std::ops::ControlFlow::{Break, Continue};
///
///     while let Some(message) = channel.next().await {
///         match message {
///             echo @ (Message::Binary(_) | Message::Text(_)) => {
///                 channel.send(echo).await.map_err(Continue)?;
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
    F: Fn(Channel, Request<State>) -> R + Send + Sync + 'static,
    R: Future<Output = Result> + Send,
{
    Upgrade::new(upgraded)
}

impl<T, E> Retry for std::result::Result<T, E>
where
    Error: From<E>,
{
    type Output = T;

    fn or_break(self) -> Result<Self::Output> {
        self.map_err(|error| ControlFlow::Break(error.into()))
    }

    fn or_continue(self) -> Result<Self::Output> {
        self.map_err(|error| ControlFlow::Continue(error.into()))
    }
}
