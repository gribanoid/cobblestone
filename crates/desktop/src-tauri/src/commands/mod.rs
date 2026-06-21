// SPDX-License-Identifier: MIT

use cobblestone_core::Store;

pub mod graph;
pub mod notes;

pub struct AppState {
    pub store: Store,
}
