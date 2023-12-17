# Variant Go Server

A highly unfinished server for Go variants implemented in full-stack Rust.

## Development

### Database
The server needs a postgres database to store user info & games in. I recommend running a simple instance with docker: 

``` sh
docker run --name go-postgres -e POSTGRES_PASSWORD=localpw -p 127.0.0.1:5432:5432 -d postgres
```

You can then either add the following in your environment variables (adjusting for your own names), or add a `.env` file to the project root.

```
DATABASE_URL=postgres://postgres:localpw@localhost/postgres
```

To create the necessary tables run:

```
cargo install diesel_cli --no-default-features --features postgres
cd server/
diesel migration run
```

### Server

Execute the server with

``` sh
cargo run -p server
```

### Client

The client uses Dioxus CLI, install it with `cargo install dioxus-cli`.

Build & run the client with

``` sh
cd client
dx serve --hot-reload
```

## Testing

Game rules use snapshot tests powered by [insta](https://docs.rs/insta/0.16.1/insta/).

# Licensing

The main project is dual licensed under MIT and Apache 2.0 licenses - the user is free to pick either one. 
Sounds are borrowed from [Katrain](https://github.com/sanderland/katrain) and fall under the MIT license.
