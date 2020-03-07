mod path;
mod util;
mod verb;

use syn::{
    parse::{Error, Parse, ParseStream},
    token::Comma,
    LitStr,
};

pub use self::{
    path::{Param, Path},
    verb::Verb,
};

thread_local! {
    static PATHS: [syn::Path; 3] = [
        syn::parse_str("::via::http").unwrap(),
        syn::parse_str("via::http").unwrap(),
        syn::parse_str("http").unwrap(),
    ];
}

pub struct Http {
    pub path: Path,
    pub verb: Verb,
}

pub struct Service {
    pub path: Option<Path>,
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Http, Error> {
        let mut verb = Verb::new();
        let path;

        if input.peek(LitStr) {
            path = input.parse()?;
        } else {
            verb = input.parse()?;
            input.parse::<Comma>()?;
            path = input.parse()?;
        }

        Ok(Http { path, verb })
    }
}

impl Parse for Service {
    fn parse(input: ParseStream) -> Result<Service, Error> {
        Ok(Service {
            path: if input.peek(LitStr) {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}
