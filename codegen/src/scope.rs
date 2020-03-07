use crate::{
    attribute::{Http, Service},
    helpers,
    route::*,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;

type Middleware = Punctuated<syn::Expr, syn::Token![,]>;

pub struct ScopeItem {
    attr: Service,
    item: syn::ItemImpl,
}

impl ScopeItem {
    pub fn new(attr: Service, item: syn::ItemImpl) -> ScopeItem {
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
                Macro(item) if helpers::is_middleware_macro(&item.mac.path) => {
                    try_expand_middleware(item)
                }
                Method(item) => try_expand_route(ty.clone(), item),
                _ => continue,
            });
        }

        quote! {
            #scope
            impl via::Service for #ty {
                fn mount(&self, endpoint: &mut via::Location) {
                    #(let mut endpoint = endpoint.at(#path);)*
                    #(#statements)*
                }
            }
        }
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

fn try_expand_route(ty: Box<syn::Type>, item: &mut syn::ImplItemMethod) -> Option<TokenStream> {
    let attr = Http::extract(&mut item.attrs)?;
    Some(RouteItem::method(ty, attr, item.clone()).expand())
}
