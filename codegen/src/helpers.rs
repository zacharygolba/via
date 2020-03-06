thread_local! {
    static EXPOSE: [syn::Path; 6] = [
        syn::parse_str("::via::http").unwrap(),
        syn::parse_str("via::http").unwrap(),
        syn::parse_str("http").unwrap(),
        syn::parse_str("::via::expose").unwrap(),
        syn::parse_str("via::expose").unwrap(),
        syn::parse_str("expose").unwrap(),
    ];

    static MIDDLEWARE: [syn::Path; 3] = [
        syn::parse_str("::via::middleware").unwrap(),
        syn::parse_str("via::middleware").unwrap(),
        syn::parse_str("middleware").unwrap(),
    ];
}

static METHODS: [&str; 9] = [
    "CONNECT", "DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT", "TRACE",
];

pub fn is_expose_macro(path: &syn::Path) -> bool {
    EXPOSE.with(|paths| paths.iter().any(|variant| path == variant))
}

pub fn is_middleware_macro(path: &syn::Path) -> bool {
    MIDDLEWARE.with(|paths| paths.iter().any(|variant| path == variant))
}

pub fn is_known_http_method(method: &str) -> bool {
    METHODS.contains(&method)
}
