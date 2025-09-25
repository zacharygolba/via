# tls-native

A version of the hello-world example that uses HTTP/2 and [native-tls](https://github.com/sfackler/rust-native-tls).

#### Generating Self-Signed Certificates

In order to run this example, there must be a valid identity at `./localhost.p12`. If you have `sh` and `openssl` on
your machine, you can generate a self-signed certificate that is good for 1
week by running the following command.

```sh
./get-self-signed-cert.sh
```

#### Running the Server

```
cargo run
# => Server listening at https://127.0.0.1:8080

curl -k --http2-prior-knowledge https://127.0.0.1:8080/hello/<Your Name Here>
# => Hello, <Your Name Here> (via TLS)
```
