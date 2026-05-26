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

use serde::{Deserialize, Serialize};
use tauri::State;

use cobblestone_core::{Note, Store};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub store: Store,
}

// ---------------------------------------------------------------------------
// Response types (serialisable subset of Note)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
pub struct NoteInfo {
    pub slug:     String,
    pub title:    String,
    pub modified: String,
    pub size:     u64,
    pub preview:  String,
    pub tags:     Vec<String>,
}

impl From<&Note> for NoteInfo {
    fn from(n: &Note) -> Self {
        Self {
            slug:     n.name.clone(),
            title:    n.title.clone(),
            modified: n.modified.clone(),
            size:     n.size,
            preview:  n.preview.clone(),
            tags:     n.tags.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NoteContent {
    pub slug:    String,
    pub title:   String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

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
    Ok(NoteContent { slug, title, content })
}

#[tauri::command]
pub fn save_note(slug: String, content: String, state: State<AppState>) -> Result<(), String> {
    state.store.write(&slug, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_note(title: String, state: State<AppState>) -> Result<String, String> {
    let slug = cobblestone_core::slugify(&title);
    if state.store.exists(&slug) {
        return Err(format!("Note '{}' already exists", slug));
    }
    let date    = chrono::Local::now().format("%Y-%m-%d").to_string();
    let content = format!("# {title}\n\n*Created: {date}*\n\n");
    state.store.write(&slug, &content).map_err(|e| e.to_string())?;
    Ok(slug)
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
