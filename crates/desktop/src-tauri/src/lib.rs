// SPDX-License-Identifier: GPL-3.0-or-later
//
// Cobblestone — open-source knowledge base for your private thoughts
// Copyright (C) 2026  Cobblestone Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use commands::AppState;
use cobblestone_core::Store;

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::init().expect("Failed to initialise storage at ~/.cobblestone");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { store })
        .invoke_handler(tauri::generate_handler![
            commands::list_notes,
            commands::get_note,
            commands::save_note,
            commands::create_note,
            commands::delete_note,
            commands::search_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cobblestone");
}
