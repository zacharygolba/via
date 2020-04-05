use crate::{Context, Next, Result, ResultExt};
use cookie::{Cookie, CookieJar};
use http::{HeaderMap, HeaderValue};
use std::sync::{Arc, Mutex, MutexGuard};

type Open<'a> = MutexGuard<'a, CookieJar>;

pub struct Cookies {
    state: Arc<State>,
}

enum State {
    Ready(Mutex<CookieJar>),
    Uninitialized,
}

pub async fn cookies(mut context: Context, next: Next) -> Result {
    let cookies = context.state.cookies.parse(context.request.headers())?;
    let mut response = next.call(context).await?;

    for cookie in cookies.open()?.delta() {
        response.headers_mut().append(
            http::header::SET_COOKIE,
            HeaderValue::from_str(&cookie.encoded().to_string())?,
        );
    }

    Ok(response)
}

impl Cookies {
    pub fn add(&self, cookie: Cookie<'static>) -> Result<()> {
        self.open()?.add(cookie);
        Ok(())
    }

    fn open(&self) -> Result<Open> {
        match *self.state {
            State::Ready(ref jar) => Ok(jar.try_lock().unwrap()),
            State::Uninitialized => error::bail!(
                "via::middleware::cookies must be used in order to access the cookie apis"
            ),
        }
    }

    fn parse(&mut self, headers: &HeaderMap) -> Result<Self> {
        let mut jar = CookieJar::new();

        if let State::Ready(_) = *self.state {
            error::bail!("via::middleware::cookies cannot be called more than once per request");
        }

        for header in headers.get_all(http::header::COOKIE) {
            let value = header.to_str().status(400)?;

            for cookie in value.split_terminator("; ") {
                jar.add_original(cookie.parse().status(400)?);
            }
        }

        self.state = Arc::new(State::Ready(Mutex::new(jar)));
        Ok(Cookies {
            state: Arc::clone(&self.state),
        })
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Cookies {
            state: Arc::new(State::Uninitialized),
        }
    }
}
