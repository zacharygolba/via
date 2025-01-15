# Via

Welcome to **Via**, an asynchronous web framework for Rust, designed to be simple, flexible, and efficient. With Via, you can build fast and reliable web applications using familiar Rust patterns and modern async features.

## Features

-   **Asynchronous**: Built on top of `tokio`, leveraging the full power of async programming in Rust.
-   **Lightweight**: Minimalistic API with no unnecessary abstractions or dependencies.
-   **Flexible Routing**: Simple and intuitive path parameter handling.
-   **Customizable**: Fine-grained control over requests, responses, and error handling.

## Getting Started

Add the following to dependencies section of your `Cargo.toml`:

```toml
[dependencies]
via = "2.0.0-beta.17"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Hello World Example

Below is a basic example to demonstrate how to use Via to create a simple web server that responds to requests at `/hello/:name` with a personalized greeting.

```rust
use std::process::ExitCode;
use via::middleware::error_boundary;
use via::{Next, Request, Response, Server};

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn hello(request: Request, _: Next) -> via::Result {
    // Get a reference to the path parameter `name` from the request uri.
    let name = request.param("name").percent_decode().into_result()?;

    // Send a plain text response with our greeting message.
    Response::build().text(format!("Hello, {}!", name))
}

#[tokio::main]
async fn main() -> Result<ExitCode, Error> {
    // Create a new application.
    let mut app = via::new(());

    // Include an error boundary to catch any errors that occur downstream.
    app.include(error_boundary::catch(|error, _| {
        eprintln!("Error: {}", error);
    }));

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    // Start the server.
    Server::new(app).listen(("127.0.0.1", 8080)).await
}
```

### How It Works

1. **Define a Handler**: The `hello` function is an asynchronous handler that receives a `Request` and a `Next` middleware chain. It extracts the `name` parameter from the URL and returns a `Response` with a personalized greeting.

2. **Create the Application**: Using `via::new(())`, you can create a new instance of the application. This function can also accept shared state.

3. **Define an ErrorBoundary**: Define an `ErrorBoundary` middleware to catch errors that occur downstream and convert them to a response. Middleware can be added at any depth of the route tree with the `.include(middleware)` method.

4. **Define Routes**: The `app.at("/hello/:name").respond(via::get(hello))` line adds a route that listens for GET requests on `/hello/:name`. The colon (`:`) indicates a dynamic segment in the path, which will match any value and make it available as a parameter.

5. **Start the Server**: The `Server::new(app).listen(("127.0.0.1", 8080)).await` function starts the server and listens for connections on the specified address.

### Running the Example

To run this example, `cd` in to `./examples/hello-world`, and then use `cargo run`:

```sh
cargo run
```

Visit `http://127.0.0.1:8080/hello/world` in your browser, and you should see the message "Hello, world!".

## Documentation

For more detailed information on Via's features and how to use them, please refer to the official documentation. A link will be provided in this section once the crate is published.

## Contributing

Contributions are welcome! Feel free to submit issues or pull requests on our [GitHub repository](https://github.com/zacharygolba/via).

## Inspiration

This project is inspired by [Koa](https://github.com/koajs/koa) (a web framework for Node.js).

## License

Licensed under either of

-   Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
-   MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
