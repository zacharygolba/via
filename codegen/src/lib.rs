extern crate proc_macro;

mod attribute;
mod util;

mod route;
mod scope;

use proc_macro::TokenStream;

use self::{
    attribute::{Http, Service},
    route::RouteItem,
    scope::ScopeItem,
};

#[proc_macro_attribute]
pub fn http(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as Http);
    let item = syn::parse_macro_input!(item as syn::ItemFn);

    RouteItem::new(attr, item).expand().into()
}

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = syn::parse_macro_input!(attr as Service);
    let item = syn::parse_macro_input!(item as syn::ItemImpl);

    ScopeItem::new(attr, item).expand().into()
}
