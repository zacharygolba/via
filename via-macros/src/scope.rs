use crate::{paths, route::*};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
};

type Middleware = Punctuated<syn::Expr, syn::Token![,]>;
type Mount = Punctuated<MountArm, syn::Token![,]>;

pub struct ScopeAttr {
    path: Option<syn::LitStr>,
}

pub struct ScopeItem {
    attr: ScopeAttr,
    item: syn::ItemImpl,
}

struct MountArm {
    path: syn::LitStr,
    expr: syn::Expr,
}

impl ScopeItem {
    pub fn new(attr: ScopeAttr, item: syn::ItemImpl) -> ScopeItem {
        ScopeItem { attr, item }
    }

    pub fn expand(&mut self) -> TokenStream {
        use syn::ImplItem::*;

        let mut statements = Vec::new();
        let scope = &mut self.item;
        let path = self.attr.path.iter();
        let ty = &scope.self_ty;

        for item in &mut scope.items {
            statements.extend(match item {
                Macro(item) if paths::middleware(&item.mac.path) => try_expand_middleware(item),
                Macro(item) if paths::mount(&item.mac.path) => try_expand_mount(item),
                Method(item) => try_expand_route(ty.clone(), item),
                _ => continue,
            });
        }

        quote! {
            #scope
            impl via::routing::Mount for #ty {
                fn to(&self, endpoint: &mut via::routing::Location) {
                    #(let mut endpoint = endpoint.at(#path);)*
                    #(#statements)*
                }
            }
        }
    }
}

impl Parse for MountArm {
    fn parse(input: ParseStream) -> parse::Result<MountArm> {
        Ok(MountArm {
            path: input.parse()?,
            expr: {
                input.parse::<syn::Token![=>]>()?;
                input.parse()?
            },
        })
    }
}

impl Parse for ScopeAttr {
    fn parse(input: ParseStream) -> parse::Result<ScopeAttr> {
        Ok(if input.is_empty() {
            ScopeAttr { path: None }
        } else {
            ScopeAttr {
                path: Some(input.parse()?),
            }
        })
    }
}

fn try_expand_middleware(item: &mut syn::ImplItemMacro) -> Option<TokenStream> {
    let syn::ImplItemMacro { mac, .. } = item;
    let middleware = mac
        .parse_body_with(Middleware::parse_terminated)
        .unwrap()
        .into_iter();

    Some(quote! {
        #(endpoint.middleware(#middleware);)*
    })
}

fn try_expand_mount(item: &mut syn::ImplItemMacro) -> Option<TokenStream> {
    let syn::ImplItemMacro { mac, .. } = item;
    let mount = mac.parse_body_with(Mount::parse_terminated).unwrap();
    let iter = mount.iter().map(|MountArm { path, expr }| {
        quote! {
            endpoint.at(#path).mount(#expr);
        }
    });

    Some(iter.collect())
}

fn try_expand_route(ty: Box<syn::Type>, item: &mut syn::ImplItemMethod) -> Option<TokenStream> {
    let attr = RouteAttr::extract(&mut item.attrs)?;
    Some(RouteItem::method(ty, attr, item.clone()).expand())
}
