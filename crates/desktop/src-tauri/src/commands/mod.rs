// SPDX-License-Identifier: GPL-3.0-or-later

use cobblestone_core::Store;

pub mod graph;
pub mod notes;

pub struct AppState {
    pub store: Store,
}
