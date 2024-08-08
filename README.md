# Via

Welcome to **Via**, an asynchronous web framework for Rust, designed to be simple, flexible, and efficient. With Via, you can build fast and reliable web applications using familiar Rust patterns and modern async features.

## Features

-   **Asynchronous**: Built on top of `tokio`, leveraging the full power of async programming in Rust.
-   **Lightweight**: Minimalistic API with no unnecessary abstractions or dependencies.
-   **Flexible Routing**: Simple and intuitive path parameter handling.
-   **Customizable**: Fine-grained control over requests, responses, and error handling.

## Getting Started

Currently, Via is not published to crates.io. If you wish to use Via during the early development phase, you may do so by adding the following to your `Cargo.toml`:

```toml
[dependencies]
via = { git = "https://github.com/zacharygolba/via.git", branch = "feat-multi-route-match-with-slab" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Hello World Example

Below is a basic example to demonstrate how to use Via to create a simple web server that responds to requests at `/hello/:name` with a personalized greeting.

```rust
use via::{Event, Next, Request, Response, Result};

async fn hello(request: Request, _: Next) -> Result<Response> {
    // Extract the `name` parameter from the request URI.
    let name = request.param("name").required()?;
    // Respond with a greeting.
    Response::text(format!("Hello, {}!", name)).finish()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new application.
    let mut app = via::app(());

    // Define a route that listens on /hello/:name.
    app.at("/hello/:name").respond(via::get(hello));

    // Start the server.
    app.listen(("127.0.0.1", 8080), |event| match event {
        Event::ConnectionError(error) | Event::UncaughtError(error) => {
            eprintln!("Error: {}", error);
        }
        Event::ServerReady(address) => {
            println!("Server listening at http://{}", address);
        }
    })
    .await
}
```

### How It Works

1. **Define a Handler**: The `hello` function is an asynchronous handler that receives a `Request` and a `Next` middleware chain. It extracts the `name` parameter from the URL and returns a `Response` with a personalized greeting.

2. **Create the Application**: Using `via::app(())`, you can create a new instance of the application. This function can also accept shared state.

3. **Define Routes**: The `app.at("/hello/:name").respond(via::get(hello))` line adds a route that listens for GET requests on `/hello/:name`. The colon (`:`) indicates a dynamic segment in the path, which will match any value and make it available as a parameter.

4. **Start the Server**: The `app.listen(("127.0.0.1", 8080), |event| { ... })` function starts the server and listens for connections on the specified address. The event handler allows for logging errors and confirming when the server is ready.

### Running the Example

To run this example, `cd` in to `./docs/examples/hello-world`, and then use `cargo run`:

```sh
cargo run
```

Visit `http://127.0.0.1:8080/hello/world` in your browser, and you should see the message "Hello, world!".

## Documentation

For more detailed information on Via's features and how to use them, please refer to the official documentation. A link will be provided in this section once the crate is published.

## Contributing

Contributions are welcome! Feel free to submit issues or pull requests on our [GitHub repository](https://github.com/zacharygolba/via).

## License

Via is licensed under the MIT License. See the [LICENSE](LICENSE) file for more information.
