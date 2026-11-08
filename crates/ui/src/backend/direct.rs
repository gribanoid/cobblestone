// SPDX-License-Identifier: MIT

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cobblestone_core::{ProjectsConfig, Store};
use cobblestone_plugin_host::{SharedPluginHost, bootstrap_builtins_shared};

use super::VaultBackend;
use crate::models::{NoteContent, NoteGraph, NoteInfo, VaultNode};

pub struct DirectStore {
    store: Mutex<Arc<Mutex<Store>>>,
    host: Option<SharedPluginHost>,
}

impl DirectStore {
    pub fn new(store: Store) -> Self {
        Self {
            store: Mutex::new(Arc::new(Mutex::new(store))),
            host: None,
        }
    }

    pub fn shared_with_plugins(
        store: Store,
    ) -> Result<(super::SharedBackend, SharedPluginHost), String> {
        let store = Arc::new(Mutex::new(store));
        let host =
            bootstrap_builtins_shared(Arc::clone(&store), None).map_err(|e| e.to_string())?;
        let host = Arc::new(Mutex::new(host));
        let backend = Arc::new(Self {
            store: Mutex::new(store),
            host: Some(Arc::clone(&host)),
        });
        Ok((backend, host))
    }

    fn with_store<R>(&self, f: impl FnOnce(&Store) -> Result<R, String>) -> Result<R, String> {
        let current = self.store.lock().map_err(|e| e.to_string())?;
        let store = current.lock().map_err(|e| e.to_string())?;
        f(&store)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl VaultBackend for DirectStore {
    async fn list_tree(&self) -> Result<Vec<VaultNode>, String> {
        self.with_store(|s| {
            s.list_tree()
                .map(|t| t.into_iter().map(VaultNode::from).collect())
                .map_err(|e| e.to_string())
        })
    }

    async fn get_note(&self, slug: &str) -> Result<NoteContent, String> {
        self.with_store(|store| {
            let content = store.read(slug).map_err(|e| e.to_string())?;
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
        })
    }

    async fn save_note(&self, slug: &str, content: &str) -> Result<(), String> {
        self.with_store(|s| s.write(slug, content).map_err(|e| e.to_string()))
    }

    async fn create_note(&self, title: &str, folder: Option<&str>) -> Result<String, String> {
        self.with_store(|store| {
            let slug = store
                .note_id_from_title(folder, title)
                .map_err(|e| e.to_string())?;
            if store.exists(&slug) {
                return Err(format!("Note '{slug}' already exists"));
            }
            let content = format!("# {title}\n\n");
            store.write(&slug, &content).map_err(|e| e.to_string())?;
            Ok(slug)
        })
    }

    async fn create_folder(&self, path: &str) -> Result<(), String> {
        self.with_store(|s| s.create_folder(path).map_err(|e| e.to_string()))
    }

    async fn move_note(&self, slug: &str, folder: Option<&str>) -> Result<String, String> {
        self.with_store(|s| s.move_note(slug, folder).map_err(|e| e.to_string()))
    }

    async fn move_folder(&self, path: &str, dest_parent: Option<&str>) -> Result<String, String> {
        self.with_store(|s| s.move_folder(path, dest_parent).map_err(|e| e.to_string()))
    }

    async fn delete_note(&self, slug: &str) -> Result<(), String> {
        self.with_store(|s| s.delete(slug).map_err(|e| e.to_string()))
    }

    async fn rename_note(&self, slug: &str, title: &str) -> Result<String, String> {
        self.with_store(|s| s.rename_note(slug, title).map_err(|e| e.to_string()))
    }

    async fn rename_folder(&self, path: &str, name: &str) -> Result<String, String> {
        self.with_store(|s| s.rename_folder(path, name).map_err(|e| e.to_string()))
    }

    async fn delete_folder(&self, path: &str) -> Result<(), String> {
        self.with_store(|s| s.delete_folder(path).map_err(|e| e.to_string()))
    }

    async fn search_notes(&self, query: &str) -> Result<Vec<NoteInfo>, String> {
        self.with_store(|s| {
            s.search(query)
                .map(|results| results.iter().map(|(n, _)| NoteInfo::from(n)).collect())
                .map_err(|e| e.to_string())
        })
    }

    async fn note_graph(&self, slug: &str) -> Result<NoteGraph, String> {
        self.with_store(|s| {
            s.note_graph(slug)
                .map(NoteGraph::from)
                .map_err(|e| e.to_string())
        })
    }

    async fn save_attachment(
        &self,
        folder: Option<&str>,
        filename: &str,
        bytes: &[u8],
    ) -> Result<String, String> {
        self.with_store(|s| {
            s.save_attachment(folder, filename, bytes)
                .map_err(|e| e.to_string())
        })
    }

    async fn attachment_src(&self, name: &str) -> Result<String, String> {
        let bytes = self.with_store(|s| s.read_attachment(name).map_err(|e| e.to_string()))?;
        let mime = mime_from_name(name);
        Ok(format!("data:{mime};base64,{}", base64_encode(&bytes)))
    }

    async fn delete_file(&self, path: &str) -> Result<(), String> {
        self.with_store(|s| s.delete_file(path).map_err(|e| e.to_string()))
    }

    async fn vault_label(&self) -> Result<String, String> {
        self.with_store(|s| {
            Ok(s.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| s.root.display().to_string()))
        })
    }

    async fn vault_path(&self) -> Result<String, String> {
        self.with_store(|s| Ok(s.root.display().to_string()))
    }

    async fn switch_vault(&self, path: &str) -> Result<(), String> {
        let path = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
        let mut cfg = ProjectsConfig::load().map_err(|e| e.to_string())?;
        let name = cfg
            .projects
            .iter()
            .find(|project| Path::new(&project.path).canonicalize().ok().as_ref() == Some(&path))
            .map(|project| project.name.clone())
            .ok_or_else(|| format!("Project is not registered: {}", path.display()))?;
        cfg.set_last_opened(&name);
        let new_store = Store::with_root(path).map_err(|e| e.to_string())?;
        let new_store = Arc::new(Mutex::new(new_store));
        let new_host = self
            .host
            .as_ref()
            .map(|_| bootstrap_builtins_shared(Arc::clone(&new_store), None))
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut host = self
            .host
            .as_ref()
            .map(|host| host.lock())
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        cfg.save().map_err(|e| e.to_string())?;
        *store = new_store;
        if let (Some(current), Some(next)) = (&mut host, new_host) {
            **current = next;
        }
        Ok(())
    }

    async fn list_projects(&self) -> Result<Vec<(String, String)>, String> {
        let cfg = ProjectsConfig::load().map_err(|e| e.to_string())?;
        Ok(cfg.projects.into_iter().map(|p| (p.name, p.path)).collect())
    }

    async fn add_project(&self, name: &str, path: &str) -> Result<(), String> {
        if !Path::new(path).is_absolute() {
            return Err("Enter an absolute folder path".into());
        }
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let mut cfg = ProjectsConfig::load().map_err(|e| e.to_string())?;
        cfg.add(name, path).map_err(|e| e.to_string())?;
        cfg.save().map_err(|e| e.to_string())
    }
}

fn mime_from_name(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else {
        "application/octet-stream"
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
