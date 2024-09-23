# cookies

An example that demonstrates how to use signed cookies to persist state between
requests.

## Setup Instructions

This example uses signed cookies to verify the integrity and authenticity of
the cookies that are used. In order to use signed cookies, we need to generate
a secret and store it in the `.env` file in the root directory of this example.
If you do not have a `.env` file in the directory of this example, copy the
contents of `example.env` to `.env`.

```sh
cp example.env .env
```

## Generating a New Secret

When you first setup your dev environment, you'll need to generate a new secret.
You can do so by running the following command.

```sh
./gen-secret.sh
```

It's not required that you generate your secret with the script above. Any way of
generating a cryptographically random 64 byte base64 or hex-encoded string is
sufficient.

### Production Use

In production, it's recommended that you rotate your keys relatively often. If
you run application in a cluster, be mindful that the secret _should_ be the same
for every node. If it is necessary that the secret is unique for every node, many
load balancers provide a way to ensure that every request be routed to the same
node. In any case, we consider immutable deployments with a script that rotates
the secret prior to rolling out a new application version a best practice.

```sh
./gen-secret.sh
```

## Running the Example

After you have generated a secret and added it to your `.env` file, you may start the server with the following command:

```
cargo run --release
# => Server listening at https://127.0.0.1:8080

curl -k https://127.0.0.1:8080/hello/<Your Name Here>
# => Hello, <Your Name Here>
```
