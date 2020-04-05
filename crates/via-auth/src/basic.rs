use crate::{AuthResult, Strategy};
use core::{BoxFuture, Context, Result};
use http::header::AUTHORIZATION;
use std::future::Future;

pub struct BasicStrategy<F, T, U>
where
    F: Fn(String, String) -> T + Send + Sync + 'static,
    T: Future<Output = AuthResult<U>> + Send + 'static,
    U: Send + Sync + 'static,
{
    pub(crate) login: F,
}

fn decode(encoded: &str) -> Result<String> {
    Ok(String::from_utf8(base64::decode(encoded)?)?)
}

fn parse(context: &Context) -> Option<(String, String)> {
    let header = context.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let decoded = decode(header.split_terminator("Basic").nth(1)?.trim()).ok()?;
    let mut iter = decoded.split_terminator(':');

    Some((iter.next()?.to_owned(), iter.next()?.to_owned()))
}

impl<F, T, U> Strategy for BasicStrategy<F, T, U>
where
    F: Fn(String, String) -> T + Send + Sync + 'static,
    T: Future<Output = AuthResult<U>> + Send + 'static,
    U: Send + Sync + 'static,
{
    type Future = BoxFuture<AuthResult<U>>;
    type User = U;

    fn authenticate(&self, context: &Context) -> Self::Future {
        if let Some((username, password)) = parse(context) {
            Box::pin((self.login)(username, password))
        } else {
            Box::pin(async { Ok(None) })
        }
    }
}
