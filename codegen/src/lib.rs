extern crate proc_macro;

mod attr;
mod path;
mod util;
mod verb;

use self::attr::{Endpoint, Service};
use proc_macro::TokenStream;
use syn::{ItemFn, ItemImpl};

#[proc_macro_attribute]
pub fn endpoint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemFn);
    let action = syn::parse_macro_input!(attr as Endpoint);

    util::expand(&action, &mut item).into()
}

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemImpl);
    let service = syn::parse_macro_input!(attr as Service);

    util::expand(&service, &mut item).into()
}
