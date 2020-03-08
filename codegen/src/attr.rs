use crate::{path::*, util::*, verb::*};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Error, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Block, Expr, FnArg, ImplItem, ImplItemMacro, ImplItemMethod, ItemFn, ItemImpl, LitStr,
    ReturnType, Signature, Token,
};

pub struct Http {
    pub path: Path,
    pub verb: Verb,
}

pub struct Service {
    pub path: Option<Path>,
}

fn inputs<'a, I, P>(params: P, inputs: I) -> impl Iterator<Item = TokenStream> + 'a
where
    I: Iterator<Item = &'a FnArg> + 'a,
    P: Iterator<Item = Param<'a>> + 'a,
{
    let mut params = params.peekable();

    inputs.filter_map(move |input| match input {
        FnArg::Receiver(_) => Some(quote! { state.get().unwrap() }),
        FnArg::Typed(_) if params.peek().is_some() => {
            let Param { name, .. } = params.next()?;
            Some(quote! { context.param(#name)? })
        }
        FnArg::Typed(_) => Some(quote! { context }),
    })
}

fn validate(sig: &Signature) -> Result<(), Error> {
    let msg = "the http attribute macro can only be applied to async functions and methods";

    match sig.asyncness {
        Some(_) => Ok(()),
        None => Err(Error::new(sig.span(), msg)),
    }
}

fn transform(block: &mut Block, sig: &mut Signature) -> Result<(), Error> {
    let implt = syn::parse_str::<Token![impl]>("impl")?;

    *block = syn::parse_quote! {{
        async move #block
    }};

    sig.asyncness = None;
    sig.output = match &sig.output {
        ReturnType::Default => syn::parse_quote! {
            -> #implt std::future::Future<Output = ()>
        },
        ReturnType::Type(_, ty) => syn::parse_quote! {
            -> #implt std::future::Future<Output = #ty>
        },
    };

    Ok(())
}

impl Expand<ImplItemMethod> for Http {
    fn expand(&self, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
        validate(&item.sig)?;
        transform(&mut item.block, &mut item.sig)?;

        let Http { path, verb } = self;
        let arguments = inputs(path.params(item.sig.inputs.iter()), item.sig.inputs.iter());
        let target = &item.sig.ident;

        Ok(quote! {
            location.at(#path).expose(#verb, |context: via::Context, next: via::Next| {
                let state = context.state.clone();

                async move {
                    via::Respond::respond(Self::#target(#(#arguments),*).await)
                }
            });
        })
    }
}

impl Expand<ItemFn> for Http {
    fn expand(&self, item: &mut ItemFn) -> Result<TokenStream, Error> {
        validate(&item.sig)?;
        transform(&mut item.block, &mut item.sig)?;

        let Http { path, verb } = self;
        let arguments = inputs(path.params(item.sig.inputs.iter()), item.sig.inputs.iter());
        let target = format_ident!("__via_route_fn_{}", &item.sig.ident);
        let ident = std::mem::replace(&mut item.sig.ident, target.clone());
        let vis = &item.vis;

        Ok(quote! {
            #item

            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug)]
            #vis struct #ident;

            impl via::Middleware for #ident {
                fn call(&self, context: via::Context, next: via::Next) -> via::Future {
                    Box::pin(async move {
                        via::Respond::respond(#target(#(#arguments),*).await)
                    })
                }
            }

            impl via::Service for #ident {
                fn mount(&self, location: &mut via::Location) {
                    location.at(#path).expose(#verb, *self);
                }
            }
        })
    }
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Http, Error> {
        let mut verb = Verb::new();
        let path;

        if input.peek(LitStr) {
            path = input.parse()?;
        } else {
            verb = input.parse()?;
            input.parse::<Token![,]>()?;
            path = input.parse()?;
        }

        Ok(Http { path, verb })
    }
}

impl Expand<ItemImpl> for Service {
    fn expand(&self, item: &mut ItemImpl) -> Result<TokenStream, Error> {
        let mut statements = Vec::new();
        let path = self.path.iter();
        let ty = &item.self_ty;

        for item in &mut item.items {
            if let ImplItem::Method(method) = item {
                statements.push(self.expand(method)?);
            }
        }

        Ok(quote! {
            #item

            impl via::Service for #ty {
                fn mount(&self, location: &mut via::Location) {
                    #(let mut location = location.at(#path);)*
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
