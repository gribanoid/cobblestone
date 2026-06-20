// SPDX-License-Identifier: GPL-3.0-or-later

use cobblestone_core::{Note, VaultNode};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct NoteInfo {
    pub slug: String,
    pub title: String,
    pub modified: String,
    pub size: u64,
    pub preview: String,
    pub tags: Vec<String>,
}

impl From<&Note> for NoteInfo {
    fn from(n: &Note) -> Self {
        Self {
            slug: n.name.clone(),
            title: n.title.clone(),
            modified: n.modified.clone(),
            size: n.size,
            preview: n.preview.clone(),
            tags: n.tags.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NoteContent {
    pub slug: String,
    pub title: String,
    pub content: String,
}

#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Result<Vec<NoteInfo>, String> {
    state
        .store
        .list_notes()
        .map(|notes| notes.iter().map(NoteInfo::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_note(slug: String, state: State<AppState>) -> Result<NoteContent, String> {
    let content = state.store.read(&slug).map_err(|e| e.to_string())?;
    let title = content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").to_string())
        .unwrap_or_else(|| slug.clone());
    Ok(NoteContent {
        slug,
        title,
        content,
    })
}

#[tauri::command]
pub fn save_note(slug: String, content: String, state: State<AppState>) -> Result<(), String> {
    state
        .store
        .write(&slug, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tree(state: State<AppState>) -> Result<Vec<VaultNode>, String> {
    state.store.list_tree().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_folder(path: String, state: State<AppState>) -> Result<(), String> {
    state.store.create_folder(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_note(
    title: String,
    folder: Option<String>,
    state: State<AppState>,
) -> Result<String, String> {
    let slug = state
        .store
        .note_id_from_title(folder.as_deref(), &title)
        .map_err(|e| e.to_string())?;
    if state.store.exists(&slug) {
        return Err(format!("Note '{}' already exists", slug));
    }
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let content = format!("# {title}\n\n*Created: {date}*\n\n");
    state
        .store
        .write(&slug, &content)
        .map_err(|e| e.to_string())?;
    Ok(slug)
}

#[tauri::command]
pub fn move_note(
    slug: String,
    folder: Option<String>,
    state: State<AppState>,
) -> Result<String, String> {
    state
        .store
        .move_note(&slug, folder.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_folder(
    path: String,
    dest_parent: Option<String>,
    state: State<AppState>,
) -> Result<String, String> {
    state
        .store
        .move_folder(&path, dest_parent.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_note(slug: String, state: State<AppState>) -> Result<(), String> {
    state.store.delete(&slug).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_notes(query: String, state: State<AppState>) -> Result<Vec<NoteInfo>, String> {
    state
        .store
        .search(&query)
        .map(|results| results.iter().map(|(n, _)| NoteInfo::from(n)).collect())
        .map_err(|e| e.to_string())
}
