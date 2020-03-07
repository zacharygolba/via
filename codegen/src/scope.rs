use crate::{attribute::Service, route::*, util::MacroPath};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Error, punctuated::Punctuated, ImplItemMacro, ImplItemMethod, Type};

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
            let result = match item {
                Macro(item) => try_expand_middleware(item),
                Method(item) => try_expand_http(ty.clone(), item),
                _ => continue,
            };

            match result {
                Ok(tokens) => statements.extend(tokens),
                Err(e) => return e.to_compile_error().into(),
            }
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

fn try_expand_http(ty: Box<Type>, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
    let mut iter = item.attrs.iter();
    let http = match iter.position(|attr| attr.path == MacroPath::Http) {
        Some(index) => item.attrs.remove(index).parse_args()?,
        None => return Ok(TokenStream::new()),
    };

    Ok(RouteItem::method(ty, http, item.clone()).expand())
}

fn try_expand_middleware(item: &mut ImplItemMacro) -> Result<TokenStream, Error> {
    if item.mac.path != MacroPath::Middleware {
        return Ok(TokenStream::new());
    }

    let middleware = item
        .mac
        .parse_body_with(Middleware::parse_terminated)?
        .into_iter();

    Ok(quote! {
        #(endpoint.middleware(#middleware);)*
    })
}
