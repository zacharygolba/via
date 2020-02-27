use crate::{helpers, params::*};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, FnArg, Ident, ItemFn, LitStr, Token,
};

type Methods = Punctuated<Ident, Token![|]>;

#[derive(Debug)]
pub struct RouteAttr {
    pub method: Expr,
    pub path: LitStr,
}

pub struct RouteItem {
    attr: RouteAttr,
    item: RouteKind,
}

enum RouteKind {
    Method(Box<syn::Type>, syn::ImplItemMethod),
    Fn(syn::ItemFn),
}

impl RouteItem {
    pub fn new(attr: RouteAttr, item: syn::ItemFn) -> RouteItem {
        let item = RouteKind::Fn(item);
        RouteItem { attr, item }
    }

    pub fn method(ty: Box<syn::Type>, attr: RouteAttr, item: syn::ImplItemMethod) -> RouteItem {
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

impl RouteAttr {
    pub fn extract(attrs: &mut Vec<Attribute>) -> Option<RouteAttr> {
        let index = attrs.iter().position(helpers::is_route_attr)?;
        let attr = attrs.remove(index);

        if helpers::is_method_attr(&attr) {
            let ident = attr.path.get_ident()?;
            let expr = Ident::new(&ident.to_string().to_uppercase(), ident.span());

            Some(RouteAttr {
                method: syn::parse_quote! { via::routing::Verb::#expr },
                path: attr.parse_args().unwrap(),
            })
        } else {
            Some(attr.parse_args().unwrap())
        }
    }
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let mut method = syn::parse_quote! { via::routing::Verb::all() };
        let path = input.parse()?;

        named!(input, |name| match name {
            "method" => {
                let list = Methods::parse_separated_nonempty(input)?.into_iter();
                method = syn::parse_quote! { #(via::routing::Verb::#list)|* };
            }
            _ => {
                panic!("Unknown argument {}", name);
            }
        });

        Ok(RouteAttr { method, path })
    }
}

fn expand_fn(attr: &RouteAttr, item: &mut ItemFn) -> TokenStream {
    transform_block(&mut item.block);
    transform_sig(&mut item.sig);

    let RouteAttr { method, path } = attr;

    let vis = &item.vis;
    let target = format_ident!("__via_route_fn_{}", &item.sig.ident);
    let receiver = std::mem::replace(&mut item.sig.ident, target.clone());
    let middleware = expand_fn_body(attr, &target.into(), &item.sig);

    TokenStream::from(quote! {
        #item

        #[allow(non_camel_case_types)]
        #vis struct #receiver;

        impl via::Handler for #receiver {
            fn call(&self, context: via::Context, next: via::Next) -> via::Future {
                Box::pin(#middleware)
            }
        }

        impl via::routing::Route for #receiver {
            const PATH: &'static str = #path;
            const VERB: via::routing::Verb = #method;
        }
    })
}

fn expand_method(ty: &syn::Type, attr: &RouteAttr, item: &mut syn::ImplItemMethod) -> TokenStream {
    transform_block(&mut item.block);
    transform_sig(&mut item.sig);

    let ident = &item.sig.ident;
    let target = syn::parse_quote! { #ty::#ident };

    expand_expose_fn(attr, &target, &item.sig)
}

fn expand_fn_body(attr: &RouteAttr, target: &syn::Path, receiver: &syn::Signature) -> TokenStream {
    let RouteAttr { path, .. } = attr;
    let PathArg { params, .. } = PathArg::new(path.clone(), receiver);
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

fn expand_expose_fn(
    attr: &RouteAttr,
    target: &syn::Path,
    receiver: &syn::Signature,
) -> TokenStream {
    let RouteAttr { method, path } = attr;
    let PathArg { params, .. } = PathArg::new(path.clone(), receiver);
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
        endpoint.at(#path).expose(
            #method,
            move |context: via::Context, next: via::Next| async move {
                via::Respond::respond(#target(#(#inputs),*).await)
            }
        );
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
