use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Error, Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Ident,
};

static METHODS: [&str; 9] = [
    "CONNECT", "DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT", "TRACE",
];

#[derive(Clone, Debug, Default)]
pub struct Verb(Punctuated<Name, Comma>);

#[derive(Clone, Debug)]
struct Name(Ident);

impl ToTokens for Name {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Name(ident) = self;

        tokens.extend(quote! {
            via::Verb::#ident
        })
    }
}

impl Parse for Name {
    fn parse(input: ParseStream) -> Result<Name, Error> {
        let ident = input.parse()?;

        if METHODS.iter().any(|method| ident == method) {
            Ok(Name(ident))
        } else {
            Err(Error::new(ident.span(), "unknown http method"))
        }
    }
}

impl Verb {
    pub fn new() -> Verb {
        Default::default()
    }
}

impl Parse for Verb {
    fn parse(input: ParseStream) -> Result<Verb, Error> {
        let mut list = Punctuated::new();
        let items;

        if input.peek(Ident) {
            list.push(input.parse()?);
        } else {
            syn::bracketed!(items in input);
            list = Punctuated::parse_separated_nonempty(&items)?;
        }

        Ok(Verb(list))
    }
}

impl ToTokens for Verb {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Verb(list) = self;
        let items = list.iter();

        tokens.extend(if list.is_empty() {
            quote! { via::Verb::all() }
        } else {
            quote! { #(#items)|* }
        });
    }
}
