.PHONY: help deps desktop desktop-build web web-build tui cli test fmt typecheck npm-install

help:
	@echo "Cobblestone commands:"
	@echo "  make deps           Install all build dependencies (npm + Rust)"
	@echo "  make npm-install    Alias for frontend npm install"
	@echo "  make desktop        Run the native Tauri desktop app"
	@echo "  make desktop-build  Build the desktop release bundle"
	@echo "  make web-build      Build the shared web UI (required before cb web)"
	@echo "  make web            Run the browser UI on the default port"
	@echo "  make tui            Run the terminal UI"
	@echo "  make cli ARGS='...' Run the cb CLI with arguments"
	@echo "  make test           Run workspace tests"
	@echo "  make fmt            Format Rust crates"
	@echo "  make typecheck      Typecheck frontend (requires deps)"

deps: npm-install
	cargo fetch
	@command -v cargo-tauri >/dev/null 2>&1 || cargo install tauri-cli --version "^2.0.0" --locked

npm-install:
	npm --prefix frontend ci

desktop: deps
	cd crates/desktop/src-tauri && cargo tauri dev

desktop-build: deps
	cd crates/desktop/src-tauri && cargo tauri build

web-build: deps
	npm run build:web --prefix frontend

web:
	cargo run -p cb -- web

tui:
	cargo run -p cb -- -i

cli:
	cargo run -p cb -- $(ARGS)

test:
	cargo test --workspace

fmt:
	cargo fmt --all

typecheck:
	npm run typecheck --prefix frontend
