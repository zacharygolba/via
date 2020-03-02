extern crate proc_macro;

mod helpers;
mod params;
mod route;
mod scope;

use proc_macro::TokenStream;

use self::{
    route::{RouteAttr, RouteItem},
    scope::{ScopeAttr, ScopeItem},
};

#[proc_macro_attribute]
pub fn expose(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as RouteAttr);
    let item = syn::parse_macro_input!(input as syn::ItemFn);

    RouteItem::new(attr, item).expand().into()
}

#[proc_macro_attribute]
pub fn service(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as ScopeAttr);
    let item = syn::parse_macro_input!(input as syn::ItemImpl);

    ScopeItem::new(attr, item).expand().into()
}
