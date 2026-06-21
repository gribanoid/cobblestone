// SPDX-License-Identifier: MIT

use cobblestone_core::NoteGraph;
use tauri::State;

use super::AppState;

#[tauri::command]
pub fn note_graph(slug: String, state: State<AppState>) -> Result<NoteGraph, String> {
    state
        .store
        .note_graph(&slug)
        .map_err(|e| e.to_string())
}
