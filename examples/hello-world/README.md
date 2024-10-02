# hello-world

A simple demonstration of how middleware can be used for control flow and how it can be attached at any depth of the router tree.

#### Running the Server

```
cargo run --release
# => Server listening at http://0.0.0.0:8080

curl http://0.0.0.0:8080/hello/<Your Name Here>
# => Hello, <Your Name Here>
```
