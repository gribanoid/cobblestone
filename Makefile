.PHONY: help deps desktop desktop-build macos-bundle web web-build tui cli test fmt lint clippy deny doc ci dx-install

# Where `dx` writes the bundled web app (respects CARGO_TARGET_DIR).
DX_WEB_PUBLIC = $(shell find "$(or $(CARGO_TARGET_DIR),target)" -path '*/cobblestone-web/*/web/public' -type d 2>/dev/null | head -1)

help:
	@echo "Cobblestone commands:"
	@echo "  make deps           Install Rust deps + dioxus CLI"
	@echo "  make desktop        Run the native Dioxus desktop app"
	@echo "  make desktop-build  Build the desktop release binary"
	@echo "  make macos-bundle   Build the macOS app with its Dock icon"
	@echo "  make web-build      Build the WASM web UI (required before cb web)"
	@echo "  make web            Run the browser UI on the default port"
	@echo "  make tui            Run the terminal UI"
	@echo "  make cli ARGS='...' Run the cb CLI with arguments"
	@echo "  make test           Run workspace tests"
	@echo "  make fmt            Format Rust crates"
	@echo "  make clippy         Clippy with -D warnings (matches CI)"
	@echo "  make lint           fmt check + clippy"
	@echo "  make deny           cargo-deny (advisories / licenses)"
	@echo "  make doc            rustdoc with -D warnings"
	@echo "  make ci             Local approximation of GitHub CI"

deps: dx-install
	cargo fetch

dx-install:
	@command -v dx >/dev/null 2>&1 || cargo install dioxus-cli --locked

desktop: deps
	cargo run -p cobblestone-desktop

desktop-build: deps
	cargo build -p cobblestone-desktop --release

macos-bundle: dx-install
	cd crates/desktop-app && dx bundle --macos --package-types macos --release --locked --out-dir ../../target/macos

web-build: deps
	cd crates/ui && dx build --web --release
	@out="$(DX_WEB_PUBLIC)"; \
	if [ -z "$$out" ]; then \
	  out=$$(find "$(or $(CARGO_TARGET_DIR),target)" -path '*/cobblestone-web/*/web/public' -type d 2>/dev/null | head -1); \
	fi; \
	if [ -z "$$out" ]; then echo "ERROR: dx web public dir not found"; exit 1; fi; \
	rm -rf crates/ui/dist-web && mkdir -p crates/ui/dist-web && cp -R "$$out"/. crates/ui/dist-web/; \
	echo "Web UI → crates/ui/dist-web"

web: web-build
	cargo run -p cb -- web

tui:
	cargo run -p cb -- -i

cli:
	cargo run -p cb -- $(ARGS)

test:
	cargo test -p cobblestone-core
	cargo test -p cobblestone-ui --features desktop
	cargo test -p cb

fmt:
	cargo fmt --all

clippy:
	cargo clippy -p cobblestone-core -p cb --all-targets -- -D warnings
	cargo clippy -p cobblestone-ui -p cobblestone-desktop --features desktop --all-targets -- -D warnings
	rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
	cargo clippy -p cobblestone-ui --features web --target wasm32-unknown-unknown --all-targets -- -D warnings

lint:
	cargo fmt --all -- --check
	$(MAKE) clippy

deny:
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked
	cargo deny check advisories bans sources licenses

doc:
	RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --document-private-items

ci: lint test doc
	@echo "CI checks passed (run 'make deny' separately if cargo-deny is installed)"
