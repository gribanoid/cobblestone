// SPDX-License-Identifier: MIT

//! Web entry point — built with `dx build --web` / `cargo build -p cobblestone-ui --features web --target wasm32-unknown-unknown`.

fn main() {
    #[cfg(feature = "web")]
    cobblestone_ui::launch_web();

    #[cfg(not(feature = "web"))]
    panic!("cobblestone-web requires --features web");
}
