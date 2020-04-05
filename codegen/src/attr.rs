use crate::{path::*, util::*, verb::*};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Error, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, FnArg, ImplItem, ImplItemMacro, ImplItemMethod, ItemFn, ItemImpl, LitStr, PatType, Token,
};

#[derive(Default)]
pub struct Action {
    meta: TokenStream,
    path: Path,
    verb: Verb,
}

pub struct Service {
    path: Option<Path>,
}

fn expand_argument<'a>(
    params: &mut impl Iterator<Item = Param<'a>>,
    scope: &mut Vec<TokenStream>,
    pat: &PatType,
) -> Option<TokenStream> {
    let context = syn::parse_str::<syn::Ident>("Context").unwrap();

    if pat.ty.is(&context) {
        while let Some(_) = params.next() {}
    }

    match params.next() {
        Some(Param { ident, pat, .. }) if pat.ident() == Some(ident) => {
            let name = ident.to_string();
            Some(quote! { context.params().get(#name)? })
        }
        Some(Param { ident, pat, .. }) => {
            let message = format!("expected identifer {}", ident);
            Some(Error::new(pat.span(), message).to_compile_error())
        }
        None => scope.pop(),
    }
}

fn expand_arguments<'a, I>(path: &'a Path, inputs: I) -> TokenStream
where
    I: Clone + Iterator<Item = &'a FnArg> + 'a,
{
    let mut params = path.params(inputs.clone()).peekable();
    let mut scope = vec![quote! { next }, quote! { context }];
    let argument = inputs.filter_map(move |input| match input {
        FnArg::Receiver(_) => Some(quote! { service }),
        FnArg::Typed(pat) => expand_argument(&mut params, &mut scope, pat),
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

impl Expand<ImplItemMethod> for Action {
    fn expand(&self, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
        let Action { path, verb, .. } = self;
        let mut location = quote! { location };
        let mut service = quote! {};
        let arguments = expand_arguments(path, item.sig.inputs.iter());
        let target = &item.sig.ident;

        if path != "/" {
            location = quote! { location.at(#path) };
        }

        if let Some(input) = get_mut_receiver(item.sig.inputs.iter_mut()) {
            service = quote! { let service = std::sync::Arc::clone(&service); };
            *input = syn::parse_quote! { self: std::sync::Arc<Self> };
        }

        Ok(quote! {{
            #service
            #location.expose(#verb, move |context: via::Context, next: via::Next| {
                #service
                async move {
                    via::Respond::respond(Self::#target(#arguments).await)
                }
            });
        }})
    }
}

impl Expand<ItemFn> for Action {
    fn expand(&self, item: &mut ItemFn) -> Result<TokenStream, Error> {
        let Action { meta, .. } = self;
        let ident = &item.sig.ident;
        let vis = &item.vis;

        Ok(quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy, Debug)]
            #vis struct #ident;

            #[via::service]
            impl #ident {
                #[action(#meta)]
                #item
            }
        })
    }
}

impl Parse for Action {
    fn parse(input: ParseStream) -> Result<Action, Error> {
        if input.is_empty() {
            return Ok(Default::default());
        }

        let mut path = Default::default();
        let mut verb = Verb::new();
        let meta = input.fork().parse::<TokenStream>()?;

        if input.peek(LitStr) {
            path = input.parse()?;
        } else {
            verb = input.parse()?;
        }

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            path = input.parse()?;
        }

        Ok(Action { meta, path, verb })
    }
}

impl Expand<ItemImpl> for Service {
    fn expand(&self, item: &mut ItemImpl) -> Result<TokenStream, Error> {
        let mut middleware = Vec::new();
        let mut routes = Vec::new();
        let path = self.path.iter();
        let ty = &item.self_ty;

        for item in &mut item.items {
            if let ImplItem::Macro(m) = item {
                middleware.push(self.expand(m)?);
            } else if let ImplItem::Method(m) = item {
                routes.push(self.expand(m)?);
            }
        }

        Ok(quote! {
            #item

            impl via::routing::Service for #ty {
                fn mount(self: std::sync::Arc<Self>, location: &mut via::routing::Location) {
                    #(let mut location = location.at(#path);)*
                    let service = self.clone();

                    #(#routes)*
                    #(#middleware)*
                }
            }
        })
    }
}

impl Expand<ImplItemMacro> for Service {
    fn expand(&self, item: &mut ImplItemMacro) -> Result<TokenStream, Error> {
        type List = Punctuated<Expr, Token![,]>;

        let mac = &item.mac;

        if let Some(method) = MacroPath::method(&mac.path) {
            let value = mac.parse_body_with(List::parse_terminated)?.into_iter();
            Ok(quote! { #(location.#method(#value);)* })
        } else {
            Ok(TokenStream::new())
        }
    }
}

impl Expand<ImplItemMethod> for Service {
    fn expand(&self, item: &mut ImplItemMethod) -> Result<TokenStream, Error> {
        let mut iter = item.attrs.iter();
        let option = iter.position(|attr| attr.path == MacroPath::Action);

        if let Some(index) = option {
            let input = item.attrs.remove(index);
            let mut action = if input.tokens.is_empty() {
                Default::default()
            } else {
                input.parse_args::<Action>()?
            };

            if let Some(path) = &self.path {
                action.path = path.concat(&action.path);
            }

            action.expand(item)
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
