#[macro_export]
macro_rules! resources {
    ($mod:path) => {
        (
            $crate::resources!($mod as collection),
            $crate::resources!($mod as member),
        )
    };
    ($mod:path as collection) => {{
        use $mod::{create, index};
        $crate::post(create).get(index)
    }};
    ($mod:path as member) => {{
        use $mod::{destroy, show, update};
        $crate::delete(destroy).patch(update).get(show)
    }};
    ($mod:path as $other:ident) => {{
        compile_error!(concat!(
            "incorrect rest! modifier \"",
            stringify!($other),
            "\"",
        ));
    }};
}
