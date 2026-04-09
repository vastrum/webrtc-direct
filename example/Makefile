.PHONY: build run clean

build:
	cd client-wasm && CARGO_TARGET_DIR=../../target-wasm wasm-pack build --dev --target bundler
	cd frontend && npm install && npx vite build

run: build
	cd server && cargo run

clean:
	rm -rf client-wasm/pkg frontend/dist frontend/node_modules target-wasm server/target
