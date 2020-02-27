use syn::{
    parse::{self, Parse, ParseStream},
    token::{Comma, Eq},
};

static METHOD: [&'static str; 9] = [
    "connect", "delete", "get", "head", "options", "patch", "post", "put", "trace",
];

macro_rules! methods {
    { $($name:ident),* } => ($(
        #[proc_macro_attribute]
        pub fn $name(meta: TokenStream, input: TokenStream) -> TokenStream {
            let meta = proc_macro2::TokenStream::from(meta);
            let method = quote::format_ident!("{}", stringify!($name).to_uppercase());
            route(TokenStream::from(quote! { #meta, method = #method }).into(), input)
        }
    )*);
}

macro_rules! named {
    ($input:ident, |$ident:ident| $block:expr) => {
        if $input.peek(syn::Token![,]) {
            $input.parse::<syn::Token![,]>()?;
        }

        while !$input.is_empty() {
            let ident = $input.parse::<syn::Ident>()?.to_string();
            let $ident = ident.as_str();

            $input.parse::<syn::Token![=]>()?;
            $block;

            if $input.peek(syn::Token![,]) {
                $input.parse::<syn::Token![,]>()?;
            }
        }
    };
}

pub fn is_method_attr(attr: &syn::Attribute) -> bool {
    if attr.style != syn::AttrStyle::Outer {
        return false;
    }

    match attr.path.get_ident() {
        Some(ident) => METHOD.iter().any(|method| ident == method),
        None => false,
    }
}

pub fn is_route_attr(attr: &syn::Attribute) -> bool {
    if attr.style != syn::AttrStyle::Outer {
        return false;
    }

    match attr.path.get_ident() {
        Some(ident) => ident == "route" || is_method_attr(attr),
        None => false,
    }
}
