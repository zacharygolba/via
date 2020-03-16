use crate::util::{fatal, IResult};
use nom::{
    branch::alt,
    bytes::complete::{take, take_till},
    character::complete::{char, one_of},
    combinator::{complete, map, map_res, value, verify},
    error::{VerboseError, VerboseErrorKind},
    multi::many1,
    sequence::{pair, preceded},
    Err,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{iter::Extend, str::FromStr};
use syn::{
    parse::{Error, Parse, ParseStream},
    FnArg, Ident, LitStr, Pat, Type,
};

#[derive(Clone)]
pub struct Param<'a> {
    pub ident: &'a Ident,
    pub pat: &'a Pat,
    pub ty: &'a Type,
}

#[derive(Clone, Debug)]
pub struct Path {
    params: Vec<Ident>,
    value: String,
}

#[derive(Clone, Debug)]
enum Part {
    Param(Ident),
    Literal,
}

fn hint(e: VerboseError<&str>) -> &'static str {
    match e.errors.last() {
        Some((_, VerboseErrorKind::Context("encoding"))) => {
            "path contains characters that require percent encoding"
        }
        Some((_, VerboseErrorKind::Context("param"))) => {
            "path parameter names must be a valid rust identifier"
        }
        _ => "invalid path argument",
    }
}

fn literal(input: &str) -> IResult<Part> {
    let (rest, input) = preceded(char('/'), take_till(|item| item == '/'))(input)?;
    let parser = value(Part::Literal, verify(take(input.len()), is_url_safe));

    fatal("encoding", parser)(input).map(|(_, output)| (rest, output))
}

fn param(input: &str) -> IResult<Part> {
    let recognize = preceded(pair(char('/'), one_of("*:")), take_till(|item| item == '/'));
    let (rest, input) = recognize(input)?;
    let parser = map(map_res(take(input.len()), syn::parse_str), Part::Param);

    fatal("param", parser)(input).map(|(_, output)| (rest, output))
}

fn is_url_safe(value: &str) -> bool {
    value.bytes().all(|byte| match byte {
        0x21 | 0x24..=0x3B | 0x3D | 0x40..=0x5F | 0x61..=0x7A | 0x7C | 0x7E => true,
        _ => false,
    })
}

impl Part {
    fn ident(self) -> Option<Ident> {
        if let Part::Param(ident) = self {
            Some(ident)
        } else {
            None
        }
    }
}

impl Path {
    pub fn concat(&self, other: &Path) -> Path {
        let own = self.params.iter().cloned();
        let rest = other.params.iter().cloned();

        Path {
            params: own.chain(rest).collect(),
            value: other.value.clone(),
        }
    }

    pub fn params<'a, I>(&'a self, iter: I) -> impl Iterator<Item = Param>
    where
        I: Iterator<Item = &'a FnArg>,
    {
        let mut iter = iter.peekable();

        if let Some(FnArg::Receiver(_)) = iter.peek() {
            iter.next();
        }

        iter.zip(&self.params)
            .filter_map(|(input, ident)| match input {
                FnArg::Receiver(_) => unreachable!(),
                FnArg::Typed(value) => Some(Param {
                    ident,
                    pat: &value.pat,
                    ty: &value.ty,
                }),
            })
    }
}

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Path, &'static str> {
        match complete(many1(alt((param, literal))))(input) {
            Err(Err::Error(e)) | Err(Err::Failure(e)) => Err(hint(e)),
            Err(Err::Incomplete(_)) => unreachable!(),
            Ok((_, parts)) => Ok(Path {
                params: parts.into_iter().filter_map(Part::ident).collect(),
                value: input.to_owned(),
            }),
        }
    }
}

impl Parse for Path {
    fn parse(input: ParseStream) -> Result<Path, Error> {
        let token = input.parse::<LitStr>()?;

        match token.value().parse() {
            Ok(path) => Ok(path),
            Err(msg) => Err(Error::new(token.span(), msg)),
        }
    }
}

impl PartialEq<str> for Path {
    fn eq(&self, other: &str) -> bool {
        self.value == other
    }
}

impl ToTokens for Path {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Path { value, .. } = self;

        tokens.extend(quote! {
            #value
        });
    }
}
