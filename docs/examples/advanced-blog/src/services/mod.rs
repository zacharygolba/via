macro_rules! connect(($name:ident) => {
    pub struct $name {
        pool: $crate::database::Pool,
    }

    impl $name {
        pub fn new(pool: &$crate::database::Pool) -> Self {
            Self { pool: pool.clone() }
        }
    }
});

mod api;

pub use self::api::ApiService;
