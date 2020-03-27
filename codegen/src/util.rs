use nom::error::{context, VerboseError};
use proc_macro2::TokenStream;
use std::cmp::PartialEq;
use syn::{parse::Error, Ident, Pat, Path};

pub type IResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

pub trait Expand<T> {
    fn expand(&self, item: &mut T) -> Result<TokenStream, Error>;
}

pub trait Identify {
    fn ident(&self) -> Option<&Ident>;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MacroPath {
    Action,
    Includes,
    Mount,
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

impl Identify for Pat {
    fn ident(&self) -> Option<&Ident> {
        if let Pat::Ident(value) = self {
            Some(&value.ident)
        } else {
            None
        }
    }
}

impl Identify for Path {
    fn ident(&self) -> Option<&Ident> {
        self.get_ident()
    }
}

impl MacroPath {
    pub fn method(path: &Path) -> Option<Ident> {
        if *path == MacroPath::Includes {
            Some(syn::parse_str("include").unwrap())
        } else if *path == MacroPath::Mount {
            Some(syn::parse_str("mount").unwrap())
        } else {
            None
        }
    }
}

impl PartialEq<Path> for MacroPath {
    fn eq(&self, other: &Path) -> bool {
        other.get_ident().map_or(false, |ident| match self {
            MacroPath::Action => ident == "action",
            MacroPath::Includes => ident == "includes",
            MacroPath::Mount => ident == "mount",
        })
    }
}

impl PartialEq<MacroPath> for Path {
    fn eq(&self, other: &MacroPath) -> bool {
        other == self
    }
}
