extern crate proc_macro;

#[macro_use]
mod helpers;
mod params;
mod paths;
mod route;
mod scope;

use proc_macro::TokenStream;
use quote::quote;

use self::{
    route::{RouteAttr, RouteItem},
    scope::{ScopeAttr, ScopeItem},
};

methods![connect, delete, get, head, options, patch, post, put, trace];

#[proc_macro_attribute]
pub fn route(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as RouteAttr);
    let item = syn::parse_macro_input!(input as syn::ItemFn);

    RouteItem::new(attr, item).expand().into()
}

#[proc_macro_attribute]
pub fn router(meta: TokenStream, input: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(meta as ScopeAttr);
    let item = syn::parse_macro_input!(input as syn::ItemImpl);

    ScopeItem::new(attr, item).expand().into()
}
