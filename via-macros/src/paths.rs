macro_rules! paths { [ $($ident:ident),*  $(,)* ] => {
    $(pub fn $ident(other: &syn::Path) -> bool {
        thread_local! {
            static PATHS: [syn::Path; 3] = [
                syn::parse_str(concat!("::via::", stringify!($ident))).unwrap(),
                syn::parse_str(concat!("via::", stringify!($ident))).unwrap(),
                syn::parse_str(stringify!($ident)).unwrap(),
            ];
        }

        PATHS.with(|paths| paths.iter().any(|path| path == other))
    })*
}; }

paths![connect, delete, get, head, middleware, mount, options, patch, post, put, trace];

pub fn method(path: &syn::Path) -> bool {
    const METHODS: [fn(&syn::Path) -> bool; 9] =
        [connect, delete, get, head, options, patch, post, put, trace];

    METHODS.iter().any(|method| method(path))
}
