use crate::{path::*, util::*, verb::*};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Error, Parse, ParseStream},
    punctuated::Punctuated,
    Expr, FnArg, ImplItem, ImplItemMacro, ImplItemMethod, ItemFn, ItemImpl, LitStr, Token,
};

pub struct Http {
    meta: TokenStream,
    path: Path,
    verb: Verb,
}

pub struct Service {
    path: Option<Path>,
}

fn expand_arguments<'a, I>(path: &'a Path, inputs: I) -> TokenStream
where
    I: Clone + Iterator<Item = &'a FnArg> + 'a,
{
    let mut params = path.params(inputs.clone()).peekable();
    let mut scope = vec![quote! { next }, quote! { context }];
    let argument = inputs.filter_map(move |input| match input {
        FnArg::Receiver(_) => Some(quote! { service }),
        FnArg::Typed(_) => {
            if params.peek().is_some() {
                let Param { name, .. } = params.next()?;
                Some(quote! { context.param(#name)? })
            } else {
                scope.pop()
            }
        }
    });

    quote! {
        #(#argument),*
    }
}

fn get_mut_receiver<'a, I>(mut inputs: I) -> Option<&'a mut FnArg>
where
    I: Iterator<Item = &'a mut FnArg>,
{
    if let input @ &mut FnArg::Receiver(_) = inputs.next()? {
        Some(input)
    } else {
        None
    }
}

impl Expand<ImplItemMethod> for Http {
    fn expand(&self, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
        let Http { path, verb, .. } = self;
        let mut service = quote! {};
        let arguments = expand_arguments(path, item.sig.inputs.iter());
        let target = &item.sig.ident;

        if let Some(input) = get_mut_receiver(item.sig.inputs.iter_mut()) {
            service = quote! { let service = std::sync::Arc::clone(&service); };
            *input = syn::parse_quote! { self: std::sync::Arc<Self> };
        }

        Ok(quote! {{
            #service
            location.at(#path).expose(#verb, move |context: via::Context, next: via::Next| {
                #service
                async move {
                    via::Respond::respond(Self::#target(#arguments).await)
                }
            });
        }})
    }
}

impl Expand<ItemFn> for Http {
    fn expand(&self, item: &mut ItemFn) -> Result<TokenStream, Error> {
        let Http { meta, .. } = self;
        let ident = &item.sig.ident;
        let vis = &item.vis;

        Ok(quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug)]
            #vis struct #ident;

            #[via::service]
            impl #ident {
                #[via::http(#meta)]
                #item
            }
        })
    }
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Http, Error> {
        let mut verb = Verb::new();
        let meta = input.fork().parse::<TokenStream>()?;
        let path;

        if input.peek(LitStr) {
            path = input.parse()?;
        } else {
            verb = input.parse()?;
            input.parse::<Token![,]>()?;
            path = input.parse()?;
        }

        Ok(Http { meta, path, verb })
    }
}

impl Expand<ItemImpl> for Service {
    fn expand(&self, item: &mut ItemImpl) -> Result<TokenStream, Error> {
        let mut statements = Vec::new();
        let path = self.path.iter();
        let ty = &item.self_ty;

        for item in &mut item.items {
            statements.push(if let ImplItem::Macro(m) = item {
                self.expand(m)?
            } else if let ImplItem::Method(m) = item {
                self.expand(m)?
            } else {
                continue;
            });
        }

        Ok(quote! {
            #item

            impl via::Service for #ty {
                fn mount(self: std::sync::Arc<Self>, location: &mut via::Location) {
                    #(let mut location = location.at(#path);)*
                    let service = self;
                    #(#statements)*
                }
            }
        })
    }
}

impl Expand<ImplItemMacro> for Service {
    fn expand(&self, item: &mut ImplItemMacro) -> Result<TokenStream, Error> {
        if item.mac.path != MacroPath::Middleware {
            return Ok(TokenStream::new());
        }

        let middleware = item
            .mac
            .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)?
            .into_iter();

        Ok(quote! {
            #(location.middleware(#middleware);)*
        })
    }
}

impl Expand<ImplItemMethod> for Service {
    fn expand(&self, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
        let mut iter = item.attrs.iter();
        let option = iter.position(|attr| attr.path == MacroPath::Http);

        if let Some(index) = option {
            let attr = item.attrs.remove(index);
            attr.parse_args::<Http>()?.expand(item)
        } else {
            Ok(TokenStream::new())
        }
    }
}

impl Parse for Service {
    fn parse(input: ParseStream) -> Result<Service, Error> {
        Ok(Service {
            path: if input.peek(LitStr) {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}
