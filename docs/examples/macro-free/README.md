# macro-free

This example shows the underlying APIs that are called by the macros in the event that you prefer a more traditional approach to configuring your route handlers.

#### Running the Server

```
cargo run --release
# => Server listening at http://0.0.0.0:8080

curl http://0.0.0.0:8080/hello/<Your Name Here>
# => Hello, <Your Name Here>
```
