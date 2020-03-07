use std::cmp::PartialEq;
use syn::Path;

thread_local! {
    static HTTP: [Path; 3] = [
        syn::parse_str("::via::http").unwrap(),
        syn::parse_str("via::http").unwrap(),
        syn::parse_str("http").unwrap(),
    ];

    static MIDDLEWARE: [Path; 3] = [
        syn::parse_str("::via::middleware").unwrap(),
        syn::parse_str("via::middleware").unwrap(),
        syn::parse_str("middleware").unwrap(),
    ];
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MacroPath {
    Http,
    Middleware,
}

impl PartialEq<Path> for MacroPath {
    fn eq(&self, other: &Path) -> bool {
        let cmp = |array: &[Path; 3]| array.iter().any(|path| other == path);

        match self {
            MacroPath::Http => HTTP.with(cmp),
            MacroPath::Middleware => MIDDLEWARE.with(cmp),
        }
    }
}

impl PartialEq<MacroPath> for Path {
    fn eq(&self, other: &MacroPath) -> bool {
        other == self
    }
}
