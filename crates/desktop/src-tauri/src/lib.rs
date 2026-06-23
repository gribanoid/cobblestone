// SPDX-License-Identifier: MIT

use cobblestone_core::Store;
use commands::AppState;

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::init().expect("Failed to initialise storage at ~/Documents/CobblestoneVault");

    tauri::Builder::default()
        .manage(AppState { store })
        .invoke_handler(tauri::generate_handler![
            commands::notes::list_tree,
            commands::notes::get_note,
            commands::notes::save_note,
            commands::notes::create_note,
            commands::notes::create_folder,
            commands::notes::move_note,
            commands::notes::move_folder,
            commands::notes::delete_note,
            commands::notes::rename_note,
            commands::notes::rename_folder,
            commands::notes::delete_folder,
            commands::notes::search_notes,
            commands::graph::note_graph,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Cobblestone");
}
