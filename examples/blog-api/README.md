# blog-api

### Initial Setup

Requires:

-   PostgreSQL server must be available. You can install it in a number of ways. The easiest is probably using [Postgres.app](https://postgresapp.com/).
-   diesel_cli which can be installed with cargo by running `cargo install diesel_cli`.

#### Database Setup

Make sure that your postgres server is running and available at the endpoint specified in the `.env` file located in this example app's directory. Once the database is running, execute the following commands to initialize the database that we'll be using for our simple blog API.

```
# Create an empty database to use for our blog.
# The name should match whatever is specified in the .env file.
psql -c "CREATE DATABASE blog;"

# Run all of the migrations located within the ./migrations directory.
diesel migration run
```

#### Running the Server

Once the migrations have completed successfully, we can run our server just as we would any other rust binary.

```
cargo run --release
# => Server listening at http://0.0.0.0:8080

# Fetch the list of posts as JSON
curl http://0.0.0.0:8080/api/posts

# Fetch the list of users as JSON
curl http://0.0.0.0:8080/api/users
```
