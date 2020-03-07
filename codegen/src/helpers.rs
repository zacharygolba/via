thread_local! {
    static MIDDLEWARE: [syn::Path; 3] = [
        syn::parse_str("::via::middleware").unwrap(),
        syn::parse_str("via::middleware").unwrap(),
        syn::parse_str("middleware").unwrap(),
    ];
}

pub fn is_middleware_macro(path: &syn::Path) -> bool {
    MIDDLEWARE.with(|paths| paths.iter().any(|variant| path == variant))
}
