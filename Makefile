.PHONY: help desktop desktop-build web tui cli test test-desktop fmt

help:
	@echo "Cobblestone commands:"
	@echo "  make desktop        Run the native Tauri desktop app"
	@echo "  make desktop-build  Build the desktop release bundle"
	@echo "  make web            Run the browser UI on the default port"
	@echo "  make tui            Run the terminal UI"
	@echo "  make cli ARGS='...' Run the cb CLI with arguments"
	@echo "  make test           Run workspace and desktop tests"
	@echo "  make fmt            Format Rust crates"

desktop:
	cd crates/desktop/src-tauri && cargo tauri dev

desktop-build:
	cd crates/desktop/src-tauri && cargo tauri build

web:
	cargo run -p cb -- web

tui:
	cargo run -p cb -- -i

cli:
	cargo run -p cb -- $(ARGS)

test:
	cargo test --workspace
	cd crates/desktop/src-tauri && cargo test

test-desktop:
	cd crates/desktop/src-tauri && cargo test

fmt:
	cargo fmt --all
	cd crates/desktop/src-tauri && cargo fmt
