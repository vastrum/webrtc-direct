# WebRTC Direct Demo

Project example for using webrtc-direct.
  - webrtc-direct is intended for Rust WASM deploymenet both in client and servers
  - Because of this need build step to compile client Rust into WASM for the web browser
  - Need the DTLS fingerprint cert + IP endpoint for every server to connect to
  - This could be done by embeding the IP endpoint + DTLS cert in the HTML as done by https://github.com/vastrum/vastrum-monorepo
  - Could also be done by having an API endpoint to fetch nodes as in this example project.

## Prerequisites

Install [Rust](https://rustup.rs), [Node.js](https://nodejs.org), and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```bash
# Add WASM compilation target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

## Run

```bash
make run
```

This builds the WASM client, bundles the frontend, compiles the server, and starts it. Open http://127.0.0.1:8080 in your browser.

1. Click Connect
2. Type a message and click Send
3. The server echoes it back

## How it works

```
Browser                              Server
  |                                    |
  |--- fetch /webrtc-info ------------>|  (get fingerprint + port)
  |<-- { port, fingerprint } ---------|
  |                                    |
  |--- WebRTC DTLS handshake -------->|  (direct UDP, no signaling)
  |<-- DataChannel open --------------|
  |                                    |
  |--- send("hello") ---------------->|  (echo message exchange)
  |<-- echo("hello") ----------------|
```

1. Server generates a DTLS key, starts a WebRTC echo server on port 3478 and an HTTP server on port 8080
2. Browser loads the page, fetches the DTLS fingerprint from `/webrtc-info`
3. Browser connects directly over WebRTC using only the server's IP + fingerprint
4. Messages are LZ4-compressed, length-framed, and chunked automatically

## Project structure

```
example/
├── server/          Rust binary: WebRTC echo server + HTTP (axum)
├── client-wasm/     Rust WASM crate: browser WebRTC client
├── frontend/        HTML + TypeScript UI, built with Vite
└── Makefile         Builds everything and runs the server
```

## Build steps (what `make run` does)

```bash
# 1. Build WASM client
cd client-wasm && wasm-pack build --dev --target bundler

# 2. Build frontend (inlines WASM into a single HTML file)
cd frontend && npm install && npx vite build

# 3. Run server (embeds the built HTML via include_str!)
cd server && cargo run
```
