extern crate proc_macro;

mod attribute;
mod helpers;
mod route;
mod scope;

use proc_macro::TokenStream;

use self::{
    attribute::{Http, Service},
    route::RouteItem,
    scope::ScopeItem,
};

#[proc_macro_attribute]
pub fn http(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as Http);
    let item = syn::parse_macro_input!(input as syn::ItemFn);

    RouteItem::new(attr, item).expand().into()
}

#[proc_macro_attribute]
pub fn service(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as Service);
    let item = syn::parse_macro_input!(input as syn::ItemImpl);

    ScopeItem::new(attr, item).expand().into()
}
