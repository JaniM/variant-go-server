# Variant Go Server

A highly unfinished server for Go variants implemented in full-stack Rust.

## Development

### Server

The server needs a postgres server to store user info & games in. I recommend running a simple instance with docker: 

``` sh
docker run --name go-postgres -e POSTGRES_PASSWORD=localpw -d postgres -p 127.0.0.1:5432:5432
```

You can then either add the following in your environment variables (adjusting for your own names), or add a `.env` file to the project root.

```
DATABASE_URL=postgres://postgres:localpw@localhost/postgres
```

Execute the server with

``` sh
cargo run -p server
```

### Client

The client uses wasm-pack, see installation instructions at <https://rustwasm.github.io/wasm-pack/installer/>.

Build & run the client with

``` sh
cd client
wasm-pack build --dev
cd www
npm i
npm run start
```
