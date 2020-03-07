use nom::error::{context, VerboseError};

pub type IResult<'a, T> = nom::IResult<&'a str, T, VerboseError<&'a str>>;

pub fn fatal<'a, F, T>(label: &'static str, parse: F) -> impl Fn(&'a str) -> IResult<'a, T>
where
    F: Fn(&'a str) -> IResult<'a, T>,
{
    use nom::Err::*;

    context(label, move |input| match parse(input) {
        Err(Error(e)) => Err(Failure(e)),
        result => result,
    })
}
