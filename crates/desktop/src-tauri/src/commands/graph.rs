// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub struct LinkedNote {
    pub slug: String,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct NoteGraph {
    pub outgoing: Vec<LinkedNote>,
    pub backlinks: Vec<LinkedNote>,
}

#[tauri::command]
pub fn note_graph(slug: String, state: State<AppState>) -> Result<NoteGraph, String> {
    let notes = state.store.list_notes().map_err(|e| e.to_string())?;
    let current = state.store.read(&slug).map_err(|e| e.to_string())?;

    let outgoing_titles = extract_wikilinks(&current);
    let outgoing = notes
        .iter()
        .filter(|note| {
            outgoing_titles.iter().any(|link| {
                link.eq_ignore_ascii_case(&note.title) || link.eq_ignore_ascii_case(&note.name)
            })
        })
        .map(|note| LinkedNote {
            slug: note.name.clone(),
            title: note.title.clone(),
        })
        .collect();

    let current_title = notes
        .iter()
        .find(|note| note.name == slug)
        .map(|note| note.title.clone())
        .unwrap_or_else(|| slug.clone());
    let current_markers = [format!("[[{current_title}]]"), format!("[[{slug}]]")];

    let backlinks = notes
        .iter()
        .filter(|note| note.name != slug)
        .filter_map(|note| {
            let content = state.store.read(&note.name).ok()?;
            let has_backlink = current_markers
                .iter()
                .any(|marker| content.to_lowercase().contains(&marker.to_lowercase()));
            has_backlink.then(|| LinkedNote {
                slug: note.name.clone(),
                title: note.title.clone(),
            })
        })
        .collect();

    Ok(NoteGraph {
        outgoing,
        backlinks,
    })
}

fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;

    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };

        let title = rest[..end].trim();
        if !title.is_empty() && !links.iter().any(|existing: &String| existing == title) {
            links.push(title.to_string());
        }
        rest = &rest[end + 2..];
    }

    links
}
