# benchmarks

A simple app used for testing the performance of routing requests to a route handler.

#### Running the Server

```
cargo run --release
# => Server listening at http://0.0.0.0:8080

curl http://0.0.0.0:8080/text
# => Hello, world!


curl http://0.0.0.0:8080/unit
# => 204 No Content
```
