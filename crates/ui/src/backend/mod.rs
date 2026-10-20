// SPDX-License-Identifier: MIT

use std::sync::Arc;

use async_trait::async_trait;

use crate::models::{NoteContent, NoteGraph, NoteInfo, VaultNode};

#[cfg(feature = "desktop")]
mod direct;
#[cfg(feature = "web")]
mod http;

#[cfg(feature = "desktop")]
pub use direct::DirectStore;
#[cfg(feature = "web")]
pub use http::HttpApi;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait VaultBackend: Send + Sync {
    async fn list_tree(&self) -> Result<Vec<VaultNode>, String>;
    async fn get_note(&self, slug: &str) -> Result<NoteContent, String>;
    async fn save_note(&self, slug: &str, content: &str) -> Result<(), String>;
    async fn create_note(&self, title: &str, folder: Option<&str>) -> Result<String, String>;
    async fn create_folder(&self, path: &str) -> Result<(), String>;
    async fn move_note(&self, slug: &str, folder: Option<&str>) -> Result<String, String>;
    async fn move_folder(&self, path: &str, dest_parent: Option<&str>) -> Result<String, String>;
    async fn delete_note(&self, slug: &str) -> Result<(), String>;
    async fn rename_note(&self, slug: &str, title: &str) -> Result<String, String>;
    async fn rename_folder(&self, path: &str, name: &str) -> Result<String, String>;
    async fn delete_folder(&self, path: &str) -> Result<(), String>;
    async fn search_notes(&self, query: &str) -> Result<Vec<NoteInfo>, String>;
    async fn note_graph(&self, slug: &str) -> Result<NoteGraph, String>;

    /// Save raw image bytes into `folder` (vault-relative, `None` = root).
    /// Returns the vault-relative path; its file name is unique vault-wide.
    async fn save_attachment(
        &self,
        folder: Option<&str>,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, String>;

    /// Browser-usable `src` for an embed target (bare name or vault path).
    async fn attachment_src(&self, name: &str) -> Result<String, String>;

    /// Delete an attachment by vault-relative path.
    async fn delete_file(&self, path: &str) -> Result<(), String>;
}

pub type SharedBackend = Arc<dyn VaultBackend>;
