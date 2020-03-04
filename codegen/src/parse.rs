use crate::helpers;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, space1},
    combinator::{map, map_res, opt, rest},
    multi::separated_list,
    sequence::pair,
    IResult,
};
use syn::{
    parse::{Parse, ParseStream, Result},
    Attribute, Expr, LitStr,
};

pub struct HttpAttr {
    pub method: Expr,
    pub path: String,
}

fn http(input: &str) -> IResult<&str, HttpAttr> {
    let path = map(rest, String::from);
    let method = map(opt(pair(methods, space1)), |option| {
        option.map_or_else(
            || syn::parse_quote! { via::verbs::Verb::all() },
            |(value, _)| value,
        )
    });

    map(pair(method, path), HttpAttr::from)(input)
}

fn methods(input: &str) -> IResult<&str, Expr> {
    let parse = |input| syn::parse_str(&format!("via::verbs::Verb::{}", input));
    let method = alt((
        tag("CONNECT"),
        tag("DELETE"),
        tag("GET"),
        tag("HEAD"),
        tag("OPTIONS"),
        tag("PATCH"),
        tag("POST"),
        tag("PUT"),
        tag("TRACE"),
    ));

    map_res(method, parse)(input)
}

impl HttpAttr {
    pub fn extract(attrs: &mut Vec<Attribute>) -> Option<HttpAttr> {
        let index = attrs
            .iter()
            .position(|attr| helpers::is_expose_macro(&attr.path))?;

        Some(attrs.remove(index).parse_args().unwrap())
    }
}

impl From<(Expr, String)> for HttpAttr {
    fn from((method, path): (Expr, String)) -> HttpAttr {
        HttpAttr { method, path }
    }
}

impl Parse for HttpAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let input = input.parse::<LitStr>()?.value();
        let (_, output) = http(&input).unwrap();

        Ok(output)
    }
}
