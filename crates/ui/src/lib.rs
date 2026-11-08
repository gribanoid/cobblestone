// SPDX-License-Identifier: MIT

//! Cobblestone graphical UI shared between desktop (Dioxus) and web (WASM).
//!
//! Architecture follows [Dioxus](https://github.com/DioxusLabs/dioxus) 0.7 idioms:
//! - `LaunchBuilder::with_context` injects [`SharedBackend`]
//! - [`state::AppCtx`] signals via `use_context_provider`
//! - [`ops::AppOp`] coroutine serializes vault mutations
//! - `asset!` + `document::Stylesheet` for CSS

pub mod actions;
pub mod app;
pub mod backend;
pub mod components;
pub mod icons;
pub mod markdown;
pub mod models;
pub mod ops;
pub mod state;
pub mod theme;
pub mod timeutil;
pub mod util;

pub use app::App;
pub use backend::{SharedBackend, VaultBackend};

/// Launch the desktop UI with a direct `Store` backend and first-party plugins.
#[cfg(feature = "desktop")]
pub fn launch_desktop(store: cobblestone_core::Store) {
    let (backend, host) =
        backend::DirectStore::shared_with_plugins(store).expect("Failed to start plugin host");
    dioxus::LaunchBuilder::desktop()
        .with_context(backend)
        .with_context(host)
        .launch(App);
}

/// Launch the web UI talking to `/api/*`.
#[cfg(feature = "web")]
pub fn launch_web() {
    let backend = backend::HttpApi::shared();
    dioxus::LaunchBuilder::web()
        .with_context(backend)
        .launch(App);
}
