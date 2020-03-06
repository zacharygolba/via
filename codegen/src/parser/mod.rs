mod http;
mod path;
mod verb;

pub use self::{http::Http, path::Path, verb::Verb};
use nom::error::{context, VerboseError};

type IResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

fn fatal<'a, F, T>(label: &'static str, parse: F) -> impl Fn(&'a str) -> IResult<'a, T>
where
    F: Fn(&'a str) -> IResult<'a, T>,
{
    use nom::Err::*;

    context(label, move |input| match parse(input) {
        Err(Error(e)) => Err(Failure(e)),
        result => result,
    })
}
