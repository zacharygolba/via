use nom::error::{context, VerboseError};
use proc_macro2::TokenStream;
use std::cmp::PartialEq;
use syn::{parse::Error, Path};

thread_local! {
    static HTTP: [Path; 3] = [
        syn::parse_str("::via::http").unwrap(),
        syn::parse_str("via::http").unwrap(),
        syn::parse_str("http").unwrap(),
    ];

    static MIDDLEWARE: [Path; 3] = [
        syn::parse_str("::via::middleware").unwrap(),
        syn::parse_str("via::middleware").unwrap(),
        syn::parse_str("middleware").unwrap(),
    ];
}

pub type IResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

pub trait Expand<T> {
    fn expand(&self, item: &mut T) -> Result<TokenStream, Error>;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MacroPath {
    Http,
    Middleware,
}

pub fn expand<T>(expander: &impl Expand<T>, item: &mut T) -> TokenStream {
    match expander.expand(item) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error(),
    }
}

pub fn fatal<'a, F, T>(label: &'static str, parser: F) -> impl Fn(&'a str) -> IResult<'a, T>
where
    F: Fn(&'a str) -> IResult<'a, T>,
{
    use nom::Err::*;

    context(label, move |input| match parser(input) {
        Err(Error(e)) => Err(Failure(e)),
        result => result,
    })
}

impl PartialEq<Path> for MacroPath {
    fn eq(&self, other: &Path) -> bool {
        let cmp = |array: &[Path; 3]| array.iter().any(|path| other == path);

        match self {
            MacroPath::Http => HTTP.with(cmp),
            MacroPath::Middleware => MIDDLEWARE.with(cmp),
        }
    }
}

impl PartialEq<MacroPath> for Path {
    fn eq(&self, other: &MacroPath) -> bool {
        other == self
    }
}
