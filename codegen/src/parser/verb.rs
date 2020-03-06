use super::{fatal, IResult};
use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while1},
    character::complete::{char, space0},
    combinator::{map, map_res, verify},
    multi::separated_list,
    sequence::{delimited, pair},
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Ident;

static VERBS: [&str; 9] = [
    "CONNECT", "DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT", "TRACE",
];

#[derive(Clone, Debug, Default)]
pub struct Verb(Vec<Value>);

#[derive(Clone, Debug)]
struct Value(Ident);

impl Value {
    fn parse(input: &str) -> IResult<Value> {
        let (rest, input) = take_while1(char::is_alphabetic)(input.trim())?;
        let validate = verify(
            map(map_res(take(input.len()), syn::parse_str), Value),
            |Value(ident)| VERBS.iter().any(|item| ident == item),
        );

        fatal("verb", validate)(input).map(|(_, output)| (rest, output))
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Value(ident) = self;

        tokens.extend(quote! {
            via::verbs::Verb::#ident
        })
    }
}

impl Verb {
    pub fn parse(input: &str) -> IResult<Verb> {
        let value = map(Value::parse, Verb::from);
        let array = delimited(
            pair(tag("["), space0),
            map(separated_list(char(','), Value::parse), Verb),
            pair(space0, tag("]")),
        );

        alt((array, value))(input)
    }
}

impl From<Value> for Verb {
    fn from(value: Value) -> Verb {
        Verb(vec![value])
    }
}

impl ToTokens for Verb {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Verb(value) = self;

        tokens.extend(if value.is_empty() {
            quote! { via::verbs::Verb::all() }
        } else {
            quote! { #(#value)|* }
        });
    }
}
