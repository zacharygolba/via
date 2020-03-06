use crate::parser::Path;
use syn::{FnArg, Ident, Pat, Signature, Type};

#[derive(Clone)]
pub struct Param {
    pub ident: Ident,
    pub name: String,
    pub ty: Box<Type>,
}

pub fn extract<'a>(path: &'a Path, iter: impl Iterator<Item = &'a FnArg>) -> Vec<Param> {
    iter.filter_map(|input| match input {
        FnArg::Receiver(_) => None,
        FnArg::Typed(input) => Some(input),
    })
    .zip(path.params.iter())
    .filter_map(|(input, ident)| match &*input.pat {
        Pat::Ident(pat) if pat.ident == *ident => Some(Param {
            ident: ident.clone(),
            name: ident.to_string(),
            ty: input.ty.clone(),
        }),
        _ => None,
    })
    .collect()
}
