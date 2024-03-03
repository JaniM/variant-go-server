# Variant Go Server

A highly unfinished server for Go variants implemented in full-stack Rust.

## Development

The whole app can be spun up with `docker compose up --build`.
After that, the client should be accessible at http://localhost:8080/

### Client

While you can just use the docker image, builds on Mac OS can be very slow.
If that ends up being an issue, try this.

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
