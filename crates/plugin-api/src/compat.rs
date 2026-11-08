// SPDX-License-Identifier: MIT
//! Core plugin host types for Cobblestone (see repository `NOTICE` for attributions).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;

impl PluginError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

// ---------------------------------------------------------------------------
// Manifest & files
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub min_app_version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TAbstractFile {
    File(TFile),
    Folder(TFolder),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TFile {
    /// Path relative to vault root, without leading slash (may include `.md`).
    pub path: String,
    pub basename: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TFolder {
    pub path: String,
    pub name: String,
}

impl TFile {
    pub fn from_vault_path(rel: &str) -> Self {
        let path = rel.trim_start_matches('/').to_string();
        let name = PathBuf::from(&path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let (basename, extension) = match name.rsplit_once('.') {
            Some((b, e)) => (b.to_string(), e.to_string()),
            None => (name, String::new()),
        };
        Self {
            path,
            basename,
            extension,
        }
    }

    /// Note id used by cobblestone-core (path without `.md`).
    pub fn note_id(&self) -> String {
        if self.extension == "md" {
            self.path
                .strip_suffix(".md")
                .unwrap_or(&self.path)
                .to_string()
        } else {
            self.path.clone()
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub type CommandCallback = Arc<dyn Fn() -> PluginResult<()> + Send + Sync>;

#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub callback: CommandCallback,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: Mutex<HashMap<String, Command>>,
}

impl CommandRegistry {
    pub fn add(&self, cmd: Command) {
        self.commands.lock().unwrap().insert(cmd.id.clone(), cmd);
    }

    pub fn remove(&self, id: &str) {
        self.commands.lock().unwrap().remove(id);
    }

    pub fn execute(&self, id: &str) -> PluginResult<()> {
        let cmd = {
            let guard = self.commands.lock().unwrap();
            guard.get(id).cloned()
        };
        match cmd {
            Some(c) => (c.callback)(),
            None => Err(PluginError::msg(format!("Unknown command '{id}'"))),
        }
    }

    pub fn list(&self) -> Vec<(String, String)> {
        self.commands
            .lock()
            .unwrap()
            .values()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Notices & post-processors
// ---------------------------------------------------------------------------

pub type NoticeFn = Arc<dyn Fn(&str) + Send + Sync>;

/// Ask the host for a string (desktop modal / CLI stdin). Returns `None` if cancelled.
pub type AskValueFn = Arc<
    dyn Fn(&str /* prompt */, &str /* default */) -> PluginResult<Option<String>> + Send + Sync,
>;

/// Processes rendered HTML (Reading view) or returns replacement HTML.
pub type MarkdownPostProcessor =
    Arc<dyn Fn(&str, &MarkdownPostProcessorContext) -> String + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct MarkdownPostProcessorContext {
    pub source_path: String,
}

// ---------------------------------------------------------------------------
// Vault / workspace / metadata traits
// ---------------------------------------------------------------------------

pub trait VaultApi: Send + Sync {
    fn root_path(&self) -> PathBuf;
    fn read(&self, path: &str) -> PluginResult<String>;
    fn write(&self, path: &str, content: &str) -> PluginResult<()>;
    fn create(&self, path: &str, content: &str) -> PluginResult<TFile>;
    fn delete(&self, path: &str) -> PluginResult<()>;
    fn rename(&self, old_path: &str, new_path: &str) -> PluginResult<()>;
    fn exists(&self, path: &str) -> bool;
    fn list_markdown_files(&self) -> PluginResult<Vec<TFile>>;
    fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf {
        let data = cobblestone_plugin_bridge::plugin_data_path(&self.root_path(), plugin_id);
        data.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    }
}

pub trait WorkspaceApi: Send + Sync {
    fn active_file_path(&self) -> Option<String>;
    fn set_active_file_path(&self, path: Option<String>);
    /// Request the host UI to open a vault-relative path (usually `*.md`).
    fn open_file(&self, path: &str) -> PluginResult<()>;
    /// Soft UI hints: `"focus-search"`, `"show-tasks"`. Default: ignore.
    fn emit_ui(&self, _hint: &str) {}
}

#[derive(Debug, Clone, Default)]
pub struct CachedMetadata {
    pub headings: Vec<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

pub trait MetadataCacheApi: Send + Sync {
    fn get_file_cache(&self, path: &str) -> Option<CachedMetadata>;
}

// ---------------------------------------------------------------------------
// Editor / MarkdownView (thin; filled in Phase 2)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EditorMode {
    #[default]
    LivePreview,
    Source,
    Reading,
}

pub trait MarkdownViewApi: Send + Sync {
    fn get_mode(&self) -> EditorMode;
    fn get_file_path(&self) -> Option<String>;
}

/// Thin editor surface for first-party plugins (selection, clipboard, insert).
pub trait EditorApi: Send + Sync {
    fn selection(&self) -> String;
    fn clipboard(&self) -> String;
    /// Queue text for the host UI to insert at the caret (Live Preview / Source).
    fn queue_insert(&self, text: &str) -> PluginResult<()>;
    fn append_to_active(&self, text: &str) -> PluginResult<()>;
}

// ---------------------------------------------------------------------------
// App facade
// ---------------------------------------------------------------------------

pub struct App {
    pub vault: Arc<dyn VaultApi>,
    pub workspace: Arc<dyn WorkspaceApi>,
    pub metadata_cache: Arc<dyn MetadataCacheApi>,
    pub editor: Arc<dyn EditorApi>,
    pub commands: Arc<CommandRegistry>,
    pub notice: NoticeFn,
    pub ask_value: AskValueFn,
    post_processors: Mutex<Vec<MarkdownPostProcessor>>,
}

impl App {
    pub fn new(
        vault: Arc<dyn VaultApi>,
        workspace: Arc<dyn WorkspaceApi>,
        metadata_cache: Arc<dyn MetadataCacheApi>,
        editor: Arc<dyn EditorApi>,
        notice: NoticeFn,
        ask_value: AskValueFn,
    ) -> Arc<Self> {
        Arc::new(Self {
            vault,
            workspace,
            metadata_cache,
            editor,
            commands: Arc::new(CommandRegistry::default()),
            notice,
            ask_value,
            post_processors: Mutex::new(Vec::new()),
        })
    }

    pub fn register_markdown_post_processor(&self, proc: MarkdownPostProcessor) {
        self.post_processors.lock().unwrap().push(proc);
    }

    pub fn clear_registrations(&self) {
        self.commands.commands.lock().unwrap().clear();
        self.post_processors.lock().unwrap().clear();
    }

    pub fn run_post_processors(&self, html: &str, ctx: &MarkdownPostProcessorContext) -> String {
        let mut out = html.to_string();
        for proc in self.post_processors.lock().unwrap().iter() {
            out = proc(&out, ctx);
        }
        out
    }

    pub fn load_data(&self, plugin_id: &str) -> PluginResult<Value> {
        let path = self.vault.plugin_data_dir(plugin_id).join("data.json");
        if !path.exists() {
            return Ok(Value::Object(Default::default()));
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save_data(&self, plugin_id: &str, data: &Value) -> PluginResult<()> {
        let dir = self.vault.plugin_data_dir(plugin_id);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("data.json");
        std::fs::write(&path, serde_json::to_string_pretty(data)?)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Plugin trait
// ---------------------------------------------------------------------------

pub trait Plugin: Send {
    fn manifest(&self) -> &PluginManifest;
    fn onload(&mut self, app: Arc<App>) -> PluginResult<()>;
    fn onunload(&mut self) -> PluginResult<()> {
        Ok(())
    }
}

/// Enabled first-party plugin ids (`{vault}/.cobblestone/plugins.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnabledPluginsState {
    #[serde(default)]
    pub enabled: Vec<String>,
}

impl EnabledPluginsState {
    pub fn path(vault_root: &std::path::Path) -> PathBuf {
        cobblestone_plugin_bridge::enabled_plugins_path(vault_root)
    }

    pub fn load(vault_root: &std::path::Path) -> PluginResult<Option<Self>> {
        let path = Self::path(vault_root);
        let legacy = cobblestone_plugin_bridge::legacy_enabled_plugins_path(vault_root);
        let read_path = if path.exists() {
            path
        } else if legacy.exists() {
            legacy
        } else {
            return Ok(None);
        };
        let raw = std::fs::read_to_string(&read_path)?;
        let mut state: Self = serde_json::from_str(&raw)?;
        cobblestone_plugin_bridge::normalize_enabled_plugin_ids(&mut state.enabled);
        Ok(Some(state))
    }

    pub fn save(&self, vault_root: &std::path::Path) -> PluginResult<()> {
        use std::io::Write;

        let path = Self::path(vault_root);
        let parent = path
            .parent()
            .ok_or_else(|| PluginError::msg("Missing plugin config directory"))?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.persist(&path).map_err(|err| err.error)?;
        Ok(())
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.iter().any(|e| e == id)
    }
}
