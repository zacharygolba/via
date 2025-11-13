use diesel::backend::Backend;
use diesel::dsl::Limit;
use diesel::query_builder::QueryFragment;
use diesel_async::methods::{ExecuteDsl, LoadQuery};
use diesel_async::return_futures::{GetResult, LoadFuture};
use diesel_async::{AsyncConnectionCore, RunQueryDsl};

pub trait DebugQueryDsl<T>: Sized {
    fn debug_execute<'a, 'b>(self, connection: &'a mut T) -> T::ExecuteFuture<'a, 'b>
    where
        T: AsyncConnectionCore + Send,
        T::Backend: Default,
        <T::Backend as Backend>::QueryBuilder: Default,
        Self: ExecuteDsl<T> + QueryFragment<T::Backend> + 'b,
    {
        debug_query(&self);
        self.execute(connection)
    }

    fn debug_first<'a, 'b, U>(self, connection: &'a mut T) -> GetResult<'a, 'b, Limit<Self>, T, U>
    where
        T: AsyncConnectionCore + Send,
        T::Backend: Default,
        <T::Backend as Backend>::QueryBuilder: Default,
        U: Send + 'a,
        Self: diesel::query_dsl::methods::LimitDsl,
        Limit<Self>: LoadQuery<'b, T, U> + QueryFragment<T::Backend> + Send + 'b,
    {
        self.limit(1).debug_result(connection)
    }

    fn debug_load<'a, 'b, U>(self, connection: &'a mut T) -> LoadFuture<'a, 'b, Self, T, U>
    where
        T: AsyncConnectionCore + Send,
        T::Backend: Default,
        <T::Backend as Backend>::QueryBuilder: Default,
        U: Send,
        Self: LoadQuery<'b, T, U> + QueryFragment<T::Backend> + 'b,
    {
        debug_query(&self);
        self.load(connection)
    }

    fn debug_result<'a, 'b, U>(self, connection: &'a mut T) -> GetResult<'a, 'b, Self, T, U>
    where
        T: AsyncConnectionCore + Send,
        T::Backend: Default,
        <T::Backend as Backend>::QueryBuilder: Default,
        U: Send + 'a,
        Self: LoadQuery<'b, T, U> + QueryFragment<T::Backend> + Send + 'b,
    {
        debug_query(&self);
        self.get_result(connection)
    }
}

impl<T, U> DebugQueryDsl<T> for U where U: RunQueryDsl<T> {}

fn debug_query<T, U>(query: &T)
where
    T: QueryFragment<U>,
    U: Backend + Default,
    U::QueryBuilder: Default,
{
    if cfg!(debug_assertions) {
        println!("\n{}", diesel::debug_query::<U, _>(query));
    }
}
