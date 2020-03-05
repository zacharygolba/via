use crate::helpers;
use nom::{
    character::complete::{space0, space1},
    combinator::rest,
    *,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{self, Parse, ParseStream},
    Attribute, LitStr, Path,
};

pub struct HttpAttr {
    pub path: String,
    pub verb: Verbs,
}

#[derive(Default)]
pub struct Verbs {
    items: Vec<Path>,
}

named!(
    http_attr<&str, HttpAttr>,
    map!(
        delimited!(space0, verbs_and_path, space0),
        |(verb, path)| HttpAttr { verb, path }
    )
);

named!(verbs_and_path<&str, (Verbs, String)>,
    pair!(
        map!(opt!(pair!(verbs, space1)), |option| {
            option.map_or_else(Verbs::default, |(verb, _)| verb)
        }),
        map!(rest, String::from)
    )
);

named!(verb<&str, Path>,
    map_res!(
        alt!(
            tag!("CONNECT") | tag!("DELETE") | tag!("GET") | tag!("HEAD") |
            tag!("OPTIONS") | tag!("PATCH") | tag!("POST") | tag!("PUT") |
            tag!("TRACE")
        ),
        |value| {
            syn::parse_str(&format!("via::verbs::Verb::{}", value))
        }
    )
);

named!(verbs<&str, Verbs>,
    map!(
        alt!(
            map!(verb, |ident| vec![ident]) |
            delimited!(
                pair!(char!('['), space0),
                separated_list!(char!(','), delimited!(space0, verb, space0)),
                pair!(space0, char!(']'))
            )
        ),
        |items| Verbs { items }
    )
);

impl HttpAttr {
    pub fn extract(attrs: &mut Vec<Attribute>) -> Option<HttpAttr> {
        let index = attrs
            .iter()
            .position(|attr| helpers::is_expose_macro(&attr.path))?;

        Some(attrs.remove(index).parse_args().unwrap())
    }
}

impl Parse for HttpAttr {
    fn parse(input: ParseStream) -> parse::Result<HttpAttr> {
        let input = input.parse::<LitStr>()?.value();
        let (_, output) = http_attr(&input).unwrap();

        Ok(output)
    }
}

impl ToTokens for Verbs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut expr = quote! {
            via::verbs::Verb::all()
        };

        if !self.items.is_empty() {
            let verb = self.items.iter();
            expr = quote! { #(#verb)|* };
        }

        tokens.extend(expr);
    }
}
