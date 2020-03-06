use super::{fatal, IResult};
use nom::{
    branch::alt,
    bytes::complete::{take, take_till},
    character::complete::{char, one_of},
    combinator::{map, map_res, value, verify},
    multi::many1,
    sequence::{pair, preceded},
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;

#[derive(Clone, Debug)]
pub struct Path {
    pub params: Vec<Ident>,
    pub value: String,
}

#[derive(Clone, Debug)]
enum Part {
    Param(Ident),
    Literal,
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
    pub fn parse(input: &str) -> IResult<Path> {
        let (_, parts) = many1(alt((param, literal)))(input)?;
        let output = Path {
            params: parts.into_iter().filter_map(Part::ident).collect(),
            value: input.to_owned(),
        };

        Ok(("", output))
    }
}

impl Default for Path {
    fn default() -> Path {
        Path {
            params: Vec::new(),
            value: "/".to_owned(),
        }
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
