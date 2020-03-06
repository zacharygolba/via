use crate::{
    params::{self, Param},
    parser::Http,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, FnArg, Ident, ItemFn, LitStr, Token,
};

type Methods = Punctuated<Ident, Token![|]>;

pub struct RouteItem {
    attr: Http,
    item: RouteKind,
}

enum RouteKind {
    Method(Box<syn::Type>, syn::ImplItemMethod),
    Fn(syn::ItemFn),
}

impl RouteItem {
    pub fn new(attr: Http, item: syn::ItemFn) -> RouteItem {
        let item = RouteKind::Fn(item);
        RouteItem { attr, item }
    }

    pub fn method(ty: Box<syn::Type>, attr: Http, item: syn::ImplItemMethod) -> RouteItem {
        let item = RouteKind::Method(ty, item);
        RouteItem { attr, item }
    }

    pub fn expand(&mut self) -> TokenStream {
        match &mut self.item {
            RouteKind::Fn(item) => expand_fn(&self.attr, item),
            RouteKind::Method(ty, item) => expand_method(&*ty, &self.attr, item),
        }
    }
}

fn expand_fn(attr: &Http, item: &mut ItemFn) -> TokenStream {
    transform_block(&mut item.block);
    transform_sig(&mut item.sig);

    let Http { path, verb } = attr;

    let vis = &item.vis;
    let target = format_ident!("__via_route_fn_{}", &item.sig.ident);
    let receiver = std::mem::replace(&mut item.sig.ident, target.clone());
    let middleware = expand_fn_body(attr, &target.into(), &item.sig);

    TokenStream::from(quote! {
        #item

        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug)]
        #vis struct #receiver;

        impl via::Handler for #receiver {
            fn call(&self, context: via::Context, next: via::Next) -> via::Future {
                Box::pin(#middleware)
            }
        }

        impl via::Service for #receiver {
            fn mount(&self, location: &mut via::Location) {
                location.at(#path).expose(#verb, *self);
            }
        }
    })
}

fn expand_method(ty: &syn::Type, attr: &Http, item: &mut syn::ImplItemMethod) -> TokenStream {
    transform_block(&mut item.block);
    transform_sig(&mut item.sig);

    let ident = &item.sig.ident;
    let target = syn::parse_quote! { #ty::#ident };

    expand_expose_fn(attr, &target, &item.sig)
}

fn expand_fn_body(attr: &Http, target: &syn::Path, receiver: &syn::Signature) -> TokenStream {
    let Http { path, .. } = attr;
    let params = params::extract(&path, receiver.inputs.iter());
    let mut iter = params.iter().peekable();
    let inputs = receiver.inputs.iter().filter_map(|input| match input {
        FnArg::Receiver(_) => Some(quote! { &self }),
        FnArg::Typed(_) if iter.peek().is_some() => {
            let Param { name, .. } = iter.next()?;
            Some(quote! { context.param(#name)? })
        }
        FnArg::Typed(_) => Some(quote! { context }),
    });

    quote! {
        async move {
            via::Respond::respond(#target(#(#inputs),*).await)
        }
    }
}

fn expand_expose_fn(attr: &Http, target: &syn::Path, receiver: &syn::Signature) -> TokenStream {
    let Http { path, verb } = attr;
    let params = params::extract(&path, receiver.inputs.iter());
    let mut iter = params.iter().peekable();
    let inputs = receiver.inputs.iter().filter_map(|input| match input {
        FnArg::Receiver(_) => Some(quote! { state.get().unwrap() }),
        FnArg::Typed(_) if iter.peek().is_some() => {
            let Param { name, .. } = iter.next()?;
            Some(quote! { context.param(#name)? })
        }
        FnArg::Typed(_) => Some(quote! { context }),
    });

    quote! {
        endpoint.at(#path).expose(#verb, |context: via::Context, next: via::Next| {
            let state = context.state.clone();

            async move {
                via::Respond::respond(#target(#(#inputs),*).await)
            }
        });
    }
}

fn transform_block(block: &mut syn::Block) {
    *block = syn::parse_quote! {{ async move #block }};
}

fn transform_sig(sig: &mut syn::Signature) {
    sig.asyncness = None;
    sig.output = match &sig.output {
        syn::ReturnType::Default => syn::parse_quote! {
            -> impl std::future::Future<Output = ()>
        },
        syn::ReturnType::Type(_, ty) => syn::parse_quote! {
            -> impl std::future::Future<Output = #ty>
        },
    };
}
