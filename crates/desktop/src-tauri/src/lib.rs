// SPDX-License-Identifier: GPL-3.0-or-later

use cobblestone_core::Store;
use commands::AppState;

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::init().expect("Failed to initialise storage at ~/.cobblestone");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { store })
        .invoke_handler(tauri::generate_handler![
            commands::notes::list_notes,
            commands::notes::get_note,
            commands::notes::save_note,
            commands::notes::create_note,
            commands::notes::delete_note,
            commands::notes::search_notes,
            commands::graph::note_graph,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cobblestone");
}
