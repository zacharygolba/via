extern crate proc_macro;

mod attr;
mod path;
mod util;
mod verb;

use self::attr::{Action, Service};
use proc_macro::TokenStream;
use syn::{ItemFn, ItemImpl};

#[proc_macro_attribute]
pub fn action(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemFn);
    let action = syn::parse_macro_input!(attr as Action);

    util::expand(&action, &mut item).into()
}

#[proc_macro_attribute]
pub fn service(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as ItemImpl);
    let service = syn::parse_macro_input!(attr as Service);

    util::expand(&service, &mut item).into()
}
