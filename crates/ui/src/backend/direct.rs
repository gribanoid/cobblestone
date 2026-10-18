// SPDX-License-Identifier: MIT

use std::sync::Arc;

use async_trait::async_trait;
use cobblestone_core::Store;

use super::VaultBackend;
use crate::models::{NoteContent, NoteGraph, NoteInfo, VaultNode};

pub struct DirectStore {
    store: Arc<Store>,
}

impl DirectStore {
    pub fn new(store: Store) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    pub fn shared(store: Store) -> super::SharedBackend {
        Arc::new(Self::new(store))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl VaultBackend for DirectStore {
    async fn list_tree(&self) -> Result<Vec<VaultNode>, String> {
        self.store
            .list_tree()
            .map(|t| t.into_iter().map(VaultNode::from).collect())
            .map_err(|e| e.to_string())
    }

    async fn get_note(&self, slug: &str) -> Result<NoteContent, String> {
        let content = self.store.read(slug).map_err(|e| e.to_string())?;
        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").to_string())
            .unwrap_or_else(|| slug.to_string());
        Ok(NoteContent {
            slug: slug.to_string(),
            title,
            content,
        })
    }

    async fn save_note(&self, slug: &str, content: &str) -> Result<(), String> {
        self.store.write(slug, content).map_err(|e| e.to_string())
    }

    async fn create_note(&self, title: &str, folder: Option<&str>) -> Result<String, String> {
        let slug = self
            .store
            .note_id_from_title(folder, title)
            .map_err(|e| e.to_string())?;
        if self.store.exists(&slug) {
            return Err(format!("Note '{slug}' already exists"));
        }
        let content = format!("# {title}\n\n");
        self.store
            .write(&slug, &content)
            .map_err(|e| e.to_string())?;
        Ok(slug)
    }

    async fn create_folder(&self, path: &str) -> Result<(), String> {
        self.store.create_folder(path).map_err(|e| e.to_string())
    }

    async fn move_note(&self, slug: &str, folder: Option<&str>) -> Result<String, String> {
        self.store
            .move_note(slug, folder)
            .map_err(|e| e.to_string())
    }

    async fn move_folder(&self, path: &str, dest_parent: Option<&str>) -> Result<String, String> {
        self.store
            .move_folder(path, dest_parent)
            .map_err(|e| e.to_string())
    }

    async fn delete_note(&self, slug: &str) -> Result<(), String> {
        self.store.delete(slug).map_err(|e| e.to_string())
    }

    async fn rename_note(&self, slug: &str, title: &str) -> Result<String, String> {
        self.store
            .rename_note(slug, title)
            .map_err(|e| e.to_string())
    }

    async fn rename_folder(&self, path: &str, name: &str) -> Result<String, String> {
        self.store
            .rename_folder(path, name)
            .map_err(|e| e.to_string())
    }

    async fn delete_folder(&self, path: &str) -> Result<(), String> {
        self.store.delete_folder(path).map_err(|e| e.to_string())
    }

    async fn search_notes(&self, query: &str) -> Result<Vec<NoteInfo>, String> {
        self.store
            .search(query)
            .map(|results| results.iter().map(|(n, _)| NoteInfo::from(n)).collect())
            .map_err(|e| e.to_string())
    }

    async fn note_graph(&self, slug: &str) -> Result<NoteGraph, String> {
        self.store
            .note_graph(slug)
            .map(NoteGraph::from)
            .map_err(|e| e.to_string())
    }
}
