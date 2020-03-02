use syn::{Attribute, Path};

thread_local! {
    static EXPOSE: [Path; 3] = [
        syn::parse_str("::via::expose").unwrap(),
        syn::parse_str("via::expose").unwrap(),
        syn::parse_str("expose").unwrap(),
    ];

    static MIDDLEWARE: [Path; 3] = [
        syn::parse_str("::via::middleware").unwrap(),
        syn::parse_str("via::middleware").unwrap(),
        syn::parse_str("middleware").unwrap(),
    ];
}

pub fn is_expose_macro(path: &Path) -> bool {
    EXPOSE.with(|paths| paths.iter().any(|variant| path == variant))
}

pub fn is_middleware_macro(path: &Path) -> bool {
    MIDDLEWARE.with(|paths| paths.iter().any(|variant| path == variant))
}
