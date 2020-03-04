extern crate proc_macro;

mod helpers;
mod params;
mod parse;
mod route;
mod scope;

use proc_macro::TokenStream;
// use quote::quote;

use self::{
    parse::HttpAttr,
    route::RouteItem,
    scope::{ScopeAttr, ScopeItem},
};

// #[proc_macro_attribute]
// pub fn http(meta: TokenStream, input: TokenStream) -> TokenStream {
//     let item = syn::parse_macro_input!(input as syn::ItemFn);
//     let ident = &item.sig.ident;
//     let tokens = TokenStream::from(quote! {
//         #[allow(non_camel_case_types)]
//         #[derive(Clone, Copy, Debug)]
//         struct #ident;

//         #[via::service]
//         impl #ident {
//             #item
//         }
//     });

//     service(meta, tokens)
// }

#[proc_macro_attribute]
pub fn http(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as HttpAttr);
    let item = syn::parse_macro_input!(input as syn::ItemFn);

    RouteItem::new(attr, item).expand().into()
}

#[proc_macro_attribute]
pub fn service(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as ScopeAttr);
    let item = syn::parse_macro_input!(input as syn::ItemImpl);

    ScopeItem::new(attr, item).expand().into()
}
