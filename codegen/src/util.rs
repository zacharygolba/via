use nom::error::{context, VerboseError};
use proc_macro2::TokenStream;
use std::cmp::PartialEq;
use syn::{parse::Error, Ident, Path};

pub type IResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

pub trait Expand<T> {
    fn expand(&self, item: &mut T) -> Result<TokenStream, Error>;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MacroPath {
    Http,
    Middleware,
    Services,
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

impl MacroPath {
    pub fn method(path: &Path) -> Option<Ident> {
        if *path == MacroPath::Middleware {
            Some(syn::parse_str("middleware").unwrap())
        } else if *path == MacroPath::Services {
            Some(syn::parse_str("service").unwrap())
        } else {
            None
        }
    }
}

impl PartialEq<Path> for MacroPath {
    fn eq(&self, other: &Path) -> bool {
        other.get_ident().map_or(false, |ident| match self {
            MacroPath::Http => ident == "http",
            MacroPath::Middleware => ident == "middleware",
            MacroPath::Services => ident == "services",
        })
    }
}

impl PartialEq<MacroPath> for Path {
    fn eq(&self, other: &MacroPath) -> bool {
        other == self
    }
}
