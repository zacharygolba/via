use juniper::RootNode;
use via::prelude::*;

#[derive(Debug)]
pub struct GraphQL<Query, Mutation, Scalar> {
    options: Options,
    schema: RootNode<Query, Mutation, Scalar>,
}

#[derive(Debug, Default)]
pub struct Options {
    graphiql: bool,
}

#[service]
impl<Query, Mutation, Scalar> GraphQL<Query, Mutation, Scalar> {
    pub fn new(schema: RootNode<Query, Mutation, Scalar>) -> GraphQL<Query, Mutation, Scalar> {
        GraphQL {
            options: Options::default(),
            schema,
        }
    }

    #[expose(POST, "/graphql")]
    async fn query(&self, context: Context) -> impl Respond {
        todo!()
    }

    #[expose(GET, "/graphiql")]
    async fn render(&self, context: Context, next: Next) -> impl Respond {
        if !self.options.graphiql {
            return next.call(context).await;
        }

        todo!()
    }
}
