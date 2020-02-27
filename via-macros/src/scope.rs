use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::route::{RouteAttr, RouteItem};

type With = Punctuated<syn::Path, syn::Token![,]>;

pub struct ScopeAttr {
    with: Vec<syn::Path>,
}

pub struct ScopeItem {
    attr: ScopeAttr,
    item: syn::ItemImpl,
}

impl ScopeItem {
    pub fn new(attr: ScopeAttr, item: syn::ItemImpl) -> ScopeItem {
        ScopeItem { attr, item }
    }

    pub fn expand(&mut self) -> TokenStream {
        let mut methods = Vec::new();
        let middleware = &self.attr.with;
        let scope = &mut self.item;
        let ty = &scope.self_ty;

        for item in &mut scope.items {
            let method = match item {
                syn::ImplItem::Method(value) => value,
                _ => continue,
            };

            let attr = match RouteAttr::extract(&mut method.attrs) {
                Some(value) => value,
                _ => continue,
            };

            methods.push(RouteItem::method(ty.clone(), attr, method.clone()).expand());
        }

        quote! {
            #scope
            impl via::routing::Scope for #ty {
                fn define(self, mut endpoint: via::routing::Location) {
                    #(endpoint.plug(#middleware());)*
                    #(#methods)*
                }
            }
        }
    }
}

impl Parse for ScopeAttr {
    fn parse(input: ParseStream) -> parse::Result<ScopeAttr> {
        let mut with = Vec::new();

        named!(input, |name| match name {
            "plug" => {
                let items;

                syn::bracketed!(items in input);
                with.extend(With::parse_separated_nonempty(&items)?);
            }
            _ => {
                panic!("Unknown argument {}", name);
            }
        });

        Ok(ScopeAttr { with })
    }
}
