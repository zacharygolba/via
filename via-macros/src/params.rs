use failure::Error;
use syn::{FnArg, Ident, LitStr, Pat, Signature, Type};

#[derive(Clone, Debug)]
pub struct PathArg {
    pub params: Vec<Param>,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub ident: Ident,
    pub name: String,
    pub ty: Box<Type>,
}

impl PathArg {
    pub fn new(lit: LitStr, sig: &Signature) -> PathArg {
        let value = lit.value();
        let params = sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                FnArg::Receiver(_) => None,
                FnArg::Typed(input) => Some(input),
            })
            .zip(value.split('/').filter_map(|part| {
                if part.starts_with('*') || part.starts_with(':') {
                    Some(part[1..].to_owned())
                } else {
                    None
                }
            }))
            .filter_map(|(input, name)| match &*input.pat {
                Pat::Ident(pat) => {
                    let ident = pat.ident.clone();
                    let ty = input.ty.clone();

                    assert_eq!(&ident, &name);
                    Some(Param { ident, name, ty })
                }
                _ => None,
            })
            .collect();

        PathArg { params, value }
    }
}
