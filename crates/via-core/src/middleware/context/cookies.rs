use crate::{BoxFuture, Context, Next, Result, ResultExt};
use cookie::{Cookie as Value, CookieBuilder, Key, SameSite};
use http::header::{self, HeaderMap};
use owning_ref::MutexGuardRef;
use std::{
    convert::TryInto,
    sync::{Arc, Mutex, MutexGuard},
};

type MasterJar = cookie::CookieJar;
type MutexJar = Mutex<MasterJar>;

pub struct Builder {
    value: CookieBuilder<'static>,
}

pub struct Cookie<'a> {
    guard: MutexGuardRef<'a, MasterJar, Value<'a>>,
}

pub struct CookieJar {
    master: Arc<MutexJar>,
    secret: Arc<Key>,
}

pub struct Middleware {
    secret: Arc<Key>,
}

pub struct PrivateJar<'a> {
    parent: &'a CookieJar,
}

pub struct SignedJar<'a> {
    parent: &'a CookieJar,
}

pub fn cookies(secret: &[u8]) -> Middleware {
    Middleware {
        secret: Key::from(secret).into(),
    }
}

fn parse(headers: &HeaderMap) -> Result<MutexJar> {
    let mut jar = cookie::CookieJar::new();

    for header in headers.get_all(header::COOKIE) {
        let value = header.to_str().status(400)?;

        for cookie in value.split_terminator("; ") {
            jar.add_original(cookie.parse().status(400)?);
        }
    }

    Ok(Mutex::new(jar))
}

impl<'a> Cookie<'a> {
    pub fn domain(&self) -> Option<&str> {
        self.guard.domain()
    }

    pub fn name(&self) -> &str {
        self.guard.name()
    }

    pub fn path(&self) -> Option<&str> {
        self.guard.path()
    }

    pub fn value(&self) -> &str {
        self.guard.value()
    }

    pub fn same_site(&self) -> Option<SameSite> {
        self.guard.same_site()
    }

    pub fn secure(&self) -> Option<bool> {
        self.guard.secure()
    }
}

impl CookieJar {
    pub fn add(&self, builder: Builder) {
        self.with(|mut master, _| {
            master.add(builder.value.finish());
        });
    }

    pub fn get<'a>(&'a self, name: &'static str) -> Option<Cookie<'a>> {
        let guard = self.read();

        guard.get(name)?;
        Some(Cookie {
            guard: guard.map(|master| master.get(name).unwrap()),
        })
    }

    pub fn remove(&self, name: &'static str) {
        self.with(|mut master, _| {
            master.remove(Value::named(name));
        });
    }
}

impl CookieJar {
    pub fn private(&self) -> PrivateJar {
        PrivateJar { parent: self }
    }

    pub fn signed(&self) -> SignedJar {
        SignedJar { parent: self }
    }
}

impl CookieJar {
    fn new(context: &mut Context, secret: Arc<Key>) -> Result<Self> {
        let master = parse(context.request.headers())?.into();
        let jar = CookieJar { master, secret };

        context.state.cookies = Some(CookieJar {
            master: Arc::clone(&jar.master),
            secret: Arc::clone(&jar.secret),
        });

        Ok(jar)
    }

    fn read<'a>(&'a self) -> MutexGuardRef<'a, MasterJar> {
        MutexGuardRef::new(self.lock())
    }

    fn lock(&self) -> MutexGuard<MasterJar> {
        self.master.try_lock().unwrap()
    }

    fn with<'a, F, T>(&'a self, f: F) -> T
    where
        F: FnOnce(MutexGuard<'a, MasterJar>, &'a Key) -> T,
    {
        f(self.master.try_lock().unwrap(), &self.secret)
    }
}

impl<'a> PrivateJar<'a> {
    pub fn add(&self, builder: Builder) {
        self.parent.with(|mut master, secret| {
            let value = builder.value.finish();
            master.private(secret).add(value);
        });
    }

    // pub fn get(&self, name: &'static str) -> Cookie<'a> {
    //     self.parent.with(|master, secret| Cookie {
    //         jar: Source::Private(master),
    //         name,
    //         secret,
    //     })
    // }
}

impl<'a> SignedJar<'a> {
    pub fn add(&self, builder: Builder) {
        self.parent.with(|mut master, secret| {
            let value = builder.value.finish();
            master.signed(secret).add(value);
        });
    }

    // pub fn get(&self, name: &'static str) -> Cookie<'a> {
    //     self.parent.with(|master, secret| Cookie {
    //         jar: Source::Signed(master),
    //         name,
    //         secret,
    //     })
    // }
}

impl crate::Middleware for Middleware {
    fn call(&self, mut context: Context, next: Next) -> BoxFuture<Result> {
        let secret = Arc::clone(&self.secret);

        Box::pin(async {
            let cookies = CookieJar::new(&mut context, secret)?;
            let mut response = next.call(context).await?;

            for cookie in cookies.lock().delta() {
                let value = cookie.encoded().to_string().try_into()?;
                response.headers_mut().append(header::SET_COOKIE, value);
            }

            Ok(response)
        })
    }
}
