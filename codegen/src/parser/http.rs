use super::{Path, Verb};
use ansi_term::Colour::Red;
use nom::{
    branch::alt,
    character::complete::space1,
    combinator::{complete, map},
    error::{convert_error, VerboseError, VerboseErrorKind},
    sequence::separated_pair,
    Err,
};
use std::fmt::Write;
use syn::{
    parse::{Error, Parse, ParseStream},
    Attribute, LitStr,
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

fn bail(input: &str, error: VerboseError<&str>) -> String {
    match error.errors.last() {
        Some((cause, VerboseErrorKind::Context("encoding"))) => {
            let mut output = String::new();
            let detail = "must be percent encoded";
            let title = "invalid path argument";

            hint(&mut output, input, cause, title, detail);
            output
        }
        Some((cause, VerboseErrorKind::Context("param"))) => {
            let mut output = String::new();
            let detail = "not a valid rust idententifier";
            let title = "invalid path argument";

            hint(&mut output, input, cause, title, detail);
            output
        }
        Some((cause, VerboseErrorKind::Context("verb"))) => {
            let mut output = String::new();
            let title = "unknown http method";

            hint(&mut output, input, cause, title, "");
            output
        }
        _ => convert_error(input, error),
    }
}

fn hint(output: &mut impl Write, source: &str, cause: &str, title: &str, detail: &str) {
    let highlight = format!("{} {}", "^".repeat(cause.len()), detail);
    let padding = " ".repeat(source.find(cause).unwrap_or(0) + title.len() + 9);

    writeln!(output, r#"{} "{}""#, title, source).unwrap();
    write!(output, "{}{}", padding, Red.paint(highlight)).unwrap();
}

impl Http {
    pub fn extract(attrs: &mut Vec<Attribute>) -> Option<Http> {
        let index = attrs
            .iter()
            .map(|attr| &attr.path)
            .position(|path| PATHS.with(|paths| paths.iter().any(|item| path == item)))?;

        match attrs.remove(index).parse_args() {
            Ok(attr) => Some(attr),
            Err(message) => panic!("{}", message),
        }
    }
}

impl From<(Verb, Path)> for Http {
    fn from((verb, path): (Verb, Path)) -> Http {
        Http { path, verb }
    }
}

impl From<Path> for Http {
    fn from(path: Path) -> Http {
        Http {
            path,
            verb: Default::default(),
        }
    }
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Http, Error> {
        let token = input.parse::<LitStr>()?;
        let input = token.value().trim().to_owned();
        let parse = complete(alt((
            map(separated_pair(Verb::parse, space1, Path::parse), Http::from),
            map(Path::parse, Http::from),
        )));

        match parse(&input) {
            Ok((_, http)) => Ok(http),
            Err(Err::Incomplete(_)) => unreachable!(),
            Err(Err::Error(error)) | Err(Err::Failure(error)) => {
                Err(Error::new(token.span(), bail(&input, error)))
            }
        }
    }
}
