.PHONY: help deps desktop desktop-build macos-bundle macos-dmg web web-build tui cli test fmt lint clippy deny doc ci dx-install

# Copy the release build, even when an older debug bundle is also present.
DX_WEB_PUBLIC = $(shell find "$(or $(CARGO_TARGET_DIR),target)" -path '*/cobblestone-web/release/web/public' -type d 2>/dev/null | head -1)

help:
	@echo "Cobblestone commands:"
	@echo "  make deps           Install Rust deps + dioxus CLI"
	@echo "  make desktop        Run the native Dioxus desktop app"
	@echo "  make desktop-build  Build the desktop release binary"
	@echo "  make macos-bundle   Build the macOS app with its Dock icon"
	@echo "  make macos-dmg      Build a drag-to-Applications disk image"
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

# Seal the complete app bundle, including its resources, before packaging.
macos-dmg: macos-bundle
	@set -eu; \
	sign_identity="$${MACOS_SIGN_IDENTITY:-}"; \
	notary_profile="$${MACOS_NOTARY_PROFILE:-}"; \
	if { [ -n "$$sign_identity" ] && [ -z "$$notary_profile" ]; } || \
	   { [ -z "$$sign_identity" ] && [ -n "$$notary_profile" ]; }; then \
		echo "Set both MACOS_SIGN_IDENTITY and MACOS_NOTARY_PROFILE to build a notarized DMG." >&2; \
		exit 1; \
	fi; \
	staging=$$(mktemp -d); \
	trap 'rm -rf "$$staging"' EXIT; \
	mkdir "$$staging/volume"; \
	app="$$staging/volume/Cobblestone.app"; \
	dmg="$$staging/Cobblestone.dmg"; \
	ditto target/macos/Cobblestone.app "$$app"; \
	if [ -n "$$sign_identity" ]; then \
		codesign --force --options runtime --timestamp --sign "$$sign_identity" "$$app"; \
	else \
		codesign --force --sign - "$$app"; \
	fi; \
	codesign --verify --deep --strict "$$app"; \
	ln -s /Applications "$$staging/volume/Applications"; \
	cp LICENSE NOTICE "$$staging/volume/"; \
	hdiutil create -volname Cobblestone -srcfolder "$$staging/volume" -format UDZO "$$dmg"; \
	if [ -n "$$sign_identity" ]; then \
		codesign --timestamp --sign "$$sign_identity" "$$dmg"; \
		xcrun notarytool submit "$$dmg" --keychain-profile "$$notary_profile" --wait; \
		xcrun stapler staple "$$dmg"; \
		xcrun stapler validate "$$dmg"; \
	fi; \
	mv "$$dmg" "target/macos/cobblestone-macos-$$(uname -m | sed 's/^arm64$$/aarch64/').dmg"

web-build: deps
	cd crates/ui && dx build --web --release --locked
	@out="$(DX_WEB_PUBLIC)"; \
	if [ -z "$$out" ]; then \
	  out=$$(find "$(or $(CARGO_TARGET_DIR),target)" -path '*/cobblestone-web/release/web/public' -type d 2>/dev/null | head -1); \
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
	cargo test --workspace --features cobblestone-ui/desktop --locked
	node --test crates/editor/tests/bridge.test.cjs

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
