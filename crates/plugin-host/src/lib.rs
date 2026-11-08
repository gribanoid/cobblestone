// SPDX-License-Identifier: MIT
//! Plugin host — bridges `cobblestone-core::Store` to `cobblestone-plugin-api`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cobblestone_core::{Store, extract_tags, extract_wikilinks};
use cobblestone_plugin_api::{
    App, AskValueFn, CachedMetadata, EditorApi, EnabledPluginsState, MarkdownPostProcessorContext,
    MetadataCacheApi, NoticeFn, Plugin, PluginError, PluginManifest, PluginResult, TFile, VaultApi,
    WorkspaceApi,
};

// ---------------------------------------------------------------------------
// Store-backed vault
// ---------------------------------------------------------------------------

pub struct StoreVault {
    store: Arc<Mutex<Store>>,
}

impl StoreVault {
    pub fn new(store: Store) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
        }
    }

    pub fn from_arc(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> Arc<Mutex<Store>> {
        Arc::clone(&self.store)
    }

    fn note_id_from_path(path: &str) -> String {
        cobblestone_plugin_bridge::to_note_id(path)
    }

    fn path_from_note_id(id: &str) -> String {
        cobblestone_plugin_bridge::to_vault_md_path(id)
    }
}

impl VaultApi for StoreVault {
    fn root_path(&self) -> PathBuf {
        self.store.lock().unwrap().root.clone()
    }

    fn read(&self, path: &str) -> PluginResult<String> {
        let id = Self::note_id_from_path(path);
        self.store
            .lock()
            .unwrap()
            .read(&id)
            .map_err(|e| PluginError::msg(e.to_string()))
    }

    fn write(&self, path: &str, content: &str) -> PluginResult<()> {
        let id = Self::note_id_from_path(path);
        self.store
            .lock()
            .unwrap()
            .write(&id, content)
            .map_err(|e| PluginError::msg(e.to_string()))
    }

    fn create(&self, path: &str, content: &str) -> PluginResult<TFile> {
        let id = Self::note_id_from_path(path);
        let store = self.store.lock().unwrap();
        if store.exists(&id) {
            return Err(PluginError::msg(format!("File already exists: {path}")));
        }
        store
            .write(&id, content)
            .map_err(|e| PluginError::msg(e.to_string()))?;
        Ok(TFile::from_vault_path(&Self::path_from_note_id(&id)))
    }

    fn delete(&self, path: &str) -> PluginResult<()> {
        let id = Self::note_id_from_path(path);
        self.store
            .lock()
            .unwrap()
            .delete(&id)
            .map_err(|e| PluginError::msg(e.to_string()))
    }

    fn rename(&self, old_path: &str, new_path: &str) -> PluginResult<()> {
        let old_id = Self::note_id_from_path(old_path);
        let new_id = Self::note_id_from_path(new_path);
        let store = self.store.lock().unwrap();
        if old_id == new_id {
            return store
                .read(&old_id)
                .map(|_| ())
                .map_err(|e| PluginError::msg(e.to_string()));
        }
        if store.exists(&new_id) {
            return Err(PluginError::msg(format!("File already exists: {new_path}")));
        }
        let content = store
            .read(&old_id)
            .map_err(|e| PluginError::msg(e.to_string()))?;
        store
            .write(&new_id, &content)
            .map_err(|e| PluginError::msg(e.to_string()))?;
        store
            .delete(&old_id)
            .map_err(|e| PluginError::msg(e.to_string()))?;
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        let id = Self::note_id_from_path(path);
        self.store.lock().unwrap().exists(&id)
    }

    fn list_markdown_files(&self) -> PluginResult<Vec<TFile>> {
        let notes = self
            .store
            .lock()
            .unwrap()
            .list_notes()
            .map_err(|e| PluginError::msg(e.to_string()))?;
        Ok(notes
            .into_iter()
            .map(|n| TFile::from_vault_path(&format!("{}.md", n.name)))
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Workspace + metadata
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct SimpleWorkspace {
    active: Mutex<Option<String>>,
    open_requests: Mutex<Vec<String>>,
    ui_actions: Mutex<Vec<HostUiAction>>,
}

/// Host → UI side effects beyond opening files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostUiAction {
    FocusSearch,
    ShowTasksPanel,
}

impl WorkspaceApi for SimpleWorkspace {
    fn active_file_path(&self) -> Option<String> {
        self.active.lock().unwrap().clone()
    }

    fn set_active_file_path(&self, path: Option<String>) {
        *self.active.lock().unwrap() = path;
    }

    fn open_file(&self, path: &str) -> PluginResult<()> {
        self.open_requests.lock().unwrap().push(path.to_string());
        *self.active.lock().unwrap() = Some(path.to_string());
        Ok(())
    }

    fn emit_ui(&self, hint: &str) {
        let action = match hint {
            "focus-search" => HostUiAction::FocusSearch,
            "show-tasks" => HostUiAction::ShowTasksPanel,
            _ => return,
        };
        self.push_ui_action(action);
    }
}

impl SimpleWorkspace {
    pub fn take_open_requests(&self) -> Vec<String> {
        std::mem::take(&mut *self.open_requests.lock().unwrap())
    }

    pub fn push_ui_action(&self, action: HostUiAction) {
        self.ui_actions.lock().unwrap().push(action);
    }

    pub fn take_ui_actions(&self) -> Vec<HostUiAction> {
        std::mem::take(&mut *self.ui_actions.lock().unwrap())
    }
}

/// Editor bridge: UI pushes selection/clipboard before a command; plugins queue inserts.
#[derive(Default)]
pub struct SimpleEditor {
    selection: Mutex<String>,
    clipboard: Mutex<String>,
    insert_queue: Mutex<Vec<String>>,
    vault: Mutex<Option<Arc<dyn VaultApi>>>,
    active: Mutex<Option<Arc<SimpleWorkspace>>>,
}

impl SimpleEditor {
    pub fn bind(&self, vault: Arc<dyn VaultApi>, workspace: Arc<SimpleWorkspace>) {
        *self.vault.lock().unwrap() = Some(vault);
        *self.active.lock().unwrap() = Some(workspace);
    }

    pub fn set_context(&self, selection: String, clipboard: String) {
        *self.selection.lock().unwrap() = selection;
        *self.clipboard.lock().unwrap() = clipboard;
    }

    pub fn take_inserts(&self) -> Vec<String> {
        std::mem::take(&mut *self.insert_queue.lock().unwrap())
    }
}

impl EditorApi for SimpleEditor {
    fn selection(&self) -> String {
        self.selection.lock().unwrap().clone()
    }

    fn clipboard(&self) -> String {
        self.clipboard.lock().unwrap().clone()
    }

    fn queue_insert(&self, text: &str) -> PluginResult<()> {
        self.insert_queue.lock().unwrap().push(text.to_string());
        Ok(())
    }

    fn append_to_active(&self, text: &str) -> PluginResult<()> {
        let workspace = self
            .active
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| PluginError::msg("Editor not bound"))?;
        let vault = self
            .vault
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| PluginError::msg("Editor not bound"))?;
        let path = workspace
            .active_file_path()
            .ok_or_else(|| PluginError::msg("No active file"))?;
        let mut content = vault.read(&path)?;
        content.push_str(text);
        vault.write(&path, &content)
    }
}

/// Non-blocking prompt: UI sets `next_value` before `execute_command`.
#[derive(Default)]
pub struct PromptBridge {
    next_value: Mutex<Option<String>>,
}

impl PromptBridge {
    pub fn set_next_value(&self, value: Option<String>) {
        *self.next_value.lock().unwrap() = value;
    }

    pub fn ask_fn(bridge: Arc<Self>) -> AskValueFn {
        Arc::new(move |prompt: &str, default: &str| {
            let _ = prompt;
            if let Some(v) = bridge.next_value.lock().unwrap().take() {
                return Ok(Some(v));
            }
            // CLI / headless: empty string means "use default", None = cancel not available.
            if default.is_empty() {
                Ok(Some(String::new()))
            } else {
                Ok(Some(default.to_string()))
            }
        })
    }
}

/// Thread-safe notice log for the desktop toast.
#[derive(Default)]
pub struct NoticeLog {
    messages: Mutex<Vec<String>>,
}

impl NoticeLog {
    pub fn push(&self, msg: &str) {
        self.messages.lock().unwrap().push(msg.to_string());
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.messages.lock().unwrap())
    }

    pub fn notice_fn(log: Arc<Self>) -> NoticeFn {
        Arc::new(move |msg: &str| {
            eprintln!("[notice] {msg}");
            log.push(msg);
        })
    }
}

pub struct CoreMetadataCache {
    vault: Arc<dyn VaultApi>,
}

impl CoreMetadataCache {
    pub fn new(vault: Arc<dyn VaultApi>) -> Self {
        Self { vault }
    }
}

impl MetadataCacheApi for CoreMetadataCache {
    fn get_file_cache(&self, path: &str) -> Option<CachedMetadata> {
        let content = self.vault.read(path).ok()?;
        let tags = extract_tags(&content);
        let links = extract_wikilinks(&content);
        let headings: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let t = line.trim_start();
                if t.starts_with('#') {
                    let text = t.trim_start_matches('#').trim();
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
                None
            })
            .collect();
        Some(CachedMetadata {
            headings,
            tags,
            links,
        })
    }
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

pub struct PluginHost {
    pub app: Arc<App>,
    pub workspace: Arc<SimpleWorkspace>,
    pub vault: Arc<StoreVault>,
    pub editor: Arc<SimpleEditor>,
    pub prompts: Arc<PromptBridge>,
    pub notices: Arc<NoticeLog>,
    plugins: Vec<Box<dyn Plugin>>,
    enabled: EnabledPluginsState,
}

impl PluginHost {
    pub fn create(store: Store, notice: Option<NoticeFn>) -> Self {
        Self::create_from_arc(Arc::new(Mutex::new(store)), notice)
    }

    /// Share the same `Store` mutex as the desktop UI backend (hot-swap safe).
    pub fn create_from_arc(store: Arc<Mutex<Store>>, notice: Option<NoticeFn>) -> Self {
        let vault = Arc::new(StoreVault::from_arc(store));
        let workspace = Arc::new(SimpleWorkspace::default());
        let editor = Arc::new(SimpleEditor::default());
        editor.bind(
            Arc::clone(&vault) as Arc<dyn VaultApi>,
            Arc::clone(&workspace),
        );
        let metadata = Arc::new(CoreMetadataCache::new(
            Arc::clone(&vault) as Arc<dyn VaultApi>
        ));
        let notices = Arc::new(NoticeLog::default());
        let notice: NoticeFn = notice.unwrap_or_else(|| NoticeLog::notice_fn(Arc::clone(&notices)));
        let prompts = Arc::new(PromptBridge::default());
        let ask_value = PromptBridge::ask_fn(Arc::clone(&prompts));
        let app = App::new(
            Arc::clone(&vault) as Arc<dyn VaultApi>,
            Arc::clone(&workspace) as Arc<dyn WorkspaceApi>,
            metadata as Arc<dyn MetadataCacheApi>,
            Arc::clone(&editor) as Arc<dyn EditorApi>,
            notice,
            ask_value,
        );
        Self {
            app,
            workspace,
            vault,
            editor,
            prompts,
            notices,
            plugins: Vec::new(),
            enabled: EnabledPluginsState::default(),
        }
    }

    pub fn set_editor_context(&self, selection: String, clipboard: String) {
        self.editor.set_context(selection, clipboard);
    }

    pub fn set_prompt_value(&self, value: Option<String>) {
        self.prompts.set_next_value(value);
    }

    pub fn take_notices(&self) -> Vec<String> {
        self.notices.take()
    }

    pub fn take_inserts(&self) -> Vec<String> {
        self.editor.take_inserts()
    }

    pub fn take_ui_actions(&self) -> Vec<HostUiAction> {
        self.workspace.take_ui_actions()
    }

    pub fn request_ui(&self, action: HostUiAction) {
        self.workspace.push_ui_action(action);
    }

    pub fn vault_root(&self) -> PathBuf {
        self.app.vault.root_path()
    }

    pub fn register_builtin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    fn load_enabled(&mut self) -> PluginResult<()> {
        self.enabled =
            EnabledPluginsState::load(&self.vault_root())?.unwrap_or_else(|| EnabledPluginsState {
                enabled: self
                    .plugins
                    .iter()
                    .map(|p| p.manifest().id.clone())
                    .collect(),
            });
        for plugin in &mut self.plugins {
            if self.enabled.is_enabled(&plugin.manifest().id) {
                plugin.onload(Arc::clone(&self.app))?;
            }
        }
        Ok(())
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        self.enabled.is_enabled(id)
    }

    pub fn list_plugins(&self) -> Vec<(PluginManifest, bool)> {
        self.plugins
            .iter()
            .map(|p| {
                let m = p.manifest().clone();
                let on = self.enabled.is_enabled(&m.id);
                (m, on)
            })
            .collect()
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> PluginResult<()> {
        let id = cobblestone_plugin_bridge::to_cobblestone_plugin_id(id);
        if !self.plugins.iter().any(|plugin| plugin.manifest().id == id) {
            return Err(PluginError::msg(format!("Unknown plugin '{id}'")));
        }
        let mut enabled = self.enabled.clone();
        if on && !enabled.is_enabled(&id) {
            enabled.enabled.push(id);
        } else if !on {
            enabled.enabled.retain(|entry| entry != &id);
        }
        enabled.save(&self.vault_root())?;
        self.enabled = enabled;
        Ok(())
    }

    pub fn execute_command(&self, id: &str) -> PluginResult<()> {
        let id = cobblestone_plugin_bridge::to_cobblestone_command_id(id);
        self.app.commands.execute(&id)
    }

    pub fn list_commands(&self) -> Vec<(String, String)> {
        self.app.commands.list()
    }

    pub fn process_html(&self, html: &str, source_path: &str) -> String {
        let ctx = MarkdownPostProcessorContext {
            source_path: source_path.to_string(),
        };
        self.app.run_post_processors(html, &ctx)
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        for plugin in &mut self.plugins {
            let _ = plugin.onunload();
        }
        // Callbacks can hold an Arc back to App.
        self.app.clear_registrations();
    }
}

/// Register all first-party plugins and load enabled ones.
pub fn bootstrap_builtins(store: Store, notice: Option<NoticeFn>) -> PluginResult<PluginHost> {
    bootstrap_builtins_shared(Arc::new(Mutex::new(store)), notice)
}

pub fn bootstrap_builtins_shared(
    store: Arc<Mutex<Store>>,
    notice: Option<NoticeFn>,
) -> PluginResult<PluginHost> {
    let mut host = PluginHost::create_from_arc(store, notice);
    host.register_builtin(Box::new(
        cobblestone_plugin_periodic_notes::PeriodicNotesPlugin::new(),
    ));
    host.register_builtin(Box::new(cobblestone_plugin_tasks::TasksPlugin::new()));
    host.register_builtin(Box::new(
        cobblestone_plugin_quick_capture::QuickCapturePlugin::new(),
    ));
    host.register_builtin(Box::new(
        cobblestone_plugin_templates_engine::DynamicTemplatesPlugin::new(),
    ));
    host.register_builtin(Box::new(
        cobblestone_plugin_omnisearch::OmniSearchPlugin::new(),
    ));
    host.load_enabled()?;
    Ok(host)
}

/// Shared host for the desktop UI.
pub type SharedPluginHost = Arc<Mutex<PluginHost>>;

#[cfg(test)]
mod tests {
    use super::*;
    use cobblestone_plugin_api::PluginManifest;

    struct TestPlugin {
        manifest: PluginManifest,
        loaded: bool,
    }

    impl Plugin for TestPlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
            self.loaded = true;
            let app2 = Arc::clone(&app);
            app.commands.add(cobblestone_plugin_api::Command {
                id: "test:ping".into(),
                name: "Ping".into(),
                callback: Arc::new(move || {
                    (app2.notice)("pong");
                    Ok(())
                }),
            });
            Ok(())
        }

        fn onunload(&mut self) -> PluginResult<()> {
            self.loaded = false;
            Ok(())
        }
    }

    #[test]
    fn host_loads_plugin_and_runs_command() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().to_path_buf()).unwrap();
        let mut host = PluginHost::create(store, None);
        host.register_builtin(Box::new(TestPlugin {
            manifest: PluginManifest {
                id: "test".into(),
                name: "Test".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: String::new(),
            },
            loaded: false,
        }));
        host.load_enabled().unwrap();
        assert!(host.enabled.is_enabled("test"));
        host.execute_command("test:ping").unwrap();
        let cmds = host.list_commands();
        assert!(cmds.iter().any(|(id, _)| id == "test:ping"));
    }

    #[test]
    fn empty_plugin_list_stays_disabled_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let store = || Store::with_root(dir.path().to_path_buf()).unwrap();
        let mut host = bootstrap_builtins(store(), None).unwrap();
        assert!(host.set_enabled("unknown", false).is_err());
        for (plugin, _) in host.list_plugins() {
            host.set_enabled(&plugin.id, false).unwrap();
        }
        drop(host);

        let host = bootstrap_builtins(store(), None).unwrap();
        assert!(host.list_plugins().iter().all(|(_, enabled)| !enabled));
        assert!(host.list_commands().is_empty());
    }

    #[test]
    fn plugin_rename_preserves_existing_notes() {
        let dir = tempfile::tempdir().unwrap();
        let vault = StoreVault::new(Store::with_root(dir.path().to_path_buf()).unwrap());
        vault.create("first.md", "first").unwrap();
        vault.create("second.md", "second").unwrap();

        vault.rename("first.md", "first.md").unwrap();
        assert!(vault.rename("first.md", "second.md").is_err());
        assert_eq!(vault.read("first.md").unwrap(), "first");
        assert_eq!(vault.read("second.md").unwrap(), "second");
    }
}
