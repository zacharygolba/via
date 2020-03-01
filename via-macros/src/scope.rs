use crate::{paths, route::*};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
};

type Middleware = Punctuated<syn::Expr, syn::Token![,]>;
type Mount = Punctuated<MountArm, syn::Token![,]>;
type With = Punctuated<syn::Path, syn::Token![,]>;

pub struct ScopeAttr {
    with: Vec<syn::Path>,
}

pub struct ScopeItem {
    attr: ScopeAttr,
    data: (Vec<TokenStream>, Vec<TokenStream>),
    item: syn::ItemImpl,
}

struct MountArm {
    path: syn::LitStr,
    expr: syn::Expr,
}

impl ScopeItem {
    pub fn new(attr: ScopeAttr, item: syn::ItemImpl) -> ScopeItem {
        let data = (vec![], vec![]);
        ScopeItem { attr, data, item }
    }

    pub fn expand(&mut self) -> TokenStream {
        use syn::ImplItem::*;

        let mut statements = Vec::new();
        let scope = &mut self.item;
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
                fn into(self, endpoint: &mut via::routing::Location) {
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
    fn parse(_: ParseStream) -> parse::Result<ScopeAttr> {
        Ok(ScopeAttr { with: Vec::new() })
    }
}

fn try_expand_middleware(item: &mut syn::ImplItemMacro) -> Option<TokenStream> {
    let syn::ImplItemMacro { mac, .. } = item;
    let middleware = mac
        .parse_body_with(Middleware::parse_terminated)
        .unwrap()
        .into_iter();

    Some(quote! {
        endpoint.plug(#(#middleware)*);
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
