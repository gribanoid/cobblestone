// SPDX-License-Identifier: MIT
//! Bridge between upstream plugin identifiers / layout conventions and
//! Cobblestone’s first-party plugin surface.
//!
//! Upstream names appear only here (mapping tables). Public Cobblestone ids stay
//! short kebab-case (`tasks`, `quick-capture`, …). See repository `NOTICE`.

use std::path::{Path, PathBuf};

/// Cobblestone config directory inside a vault.
pub const COBBLESTONE_DIR: &str = ".cobblestone";

/// Legacy upstream config directory name (read-only import / detection).
pub const UPSTREAM_CONFIG_DIR: &str = ".obsidian";

/// One known upstream → Cobblestone plugin id mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginIdMap {
    /// Historical / upstream community plugin id.
    pub upstream: &'static str,
    /// Canonical Cobblestone plugin id.
    pub cobblestone: &'static str,
}

/// Built-in id table for first-party ports.
pub const PLUGIN_ID_MAP: &[PluginIdMap] = &[
    PluginIdMap {
        upstream: "obsidian-tasks",
        cobblestone: "tasks",
    },
    PluginIdMap {
        upstream: "quickadd",
        cobblestone: "quick-capture",
    },
    PluginIdMap {
        upstream: "periodic-notes",
        cobblestone: "periodic-notes",
    },
    PluginIdMap {
        upstream: "omnisearch",
        cobblestone: "omni-search",
    },
    PluginIdMap {
        upstream: "templater-obsidian",
        cobblestone: "dynamic-templates",
    },
];

/// Map an upstream (or already-canonical) plugin id to Cobblestone.
pub fn to_cobblestone_plugin_id(id: &str) -> String {
    for m in PLUGIN_ID_MAP {
        if m.upstream == id || m.cobblestone == id {
            return m.cobblestone.to_string();
        }
    }
    id.to_string()
}

/// Best-effort reverse map (Cobblestone → preferred upstream id).
pub fn to_upstream_plugin_id(id: &str) -> String {
    for m in PLUGIN_ID_MAP {
        if m.cobblestone == id {
            return m.upstream.to_string();
        }
    }
    id.to_string()
}

/// Map a command id. Prefix before the first `:` is treated as plugin id.
///
/// `obsidian-tasks:list` → `tasks:list`
/// `tasks:list` → `tasks:list`
pub fn to_cobblestone_command_id(command_id: &str) -> String {
    match command_id.split_once(':') {
        Some((plugin, rest)) => format!("{}:{rest}", to_cobblestone_plugin_id(plugin)),
        None => to_cobblestone_plugin_id(command_id),
    }
}

pub fn to_upstream_command_id(command_id: &str) -> String {
    match command_id.split_once(':') {
        Some((plugin, rest)) => format!("{}:{rest}", to_upstream_plugin_id(plugin)),
        None => to_upstream_plugin_id(command_id),
    }
}

/// Normalize a list of enabled plugin ids in place.
pub fn normalize_enabled_plugin_ids(ids: &mut Vec<String>) {
    for id in ids.iter_mut() {
        *id = to_cobblestone_plugin_id(id);
    }
    ids.sort();
    ids.dedup();
}

/// `{vault}/.cobblestone/plugins.json`
pub fn enabled_plugins_path(vault_root: &Path) -> PathBuf {
    vault_root.join(COBBLESTONE_DIR).join("plugins.json")
}

/// Legacy Cobblestone filename (same dir).
pub fn legacy_enabled_plugins_path(vault_root: &Path) -> PathBuf {
    vault_root
        .join(COBBLESTONE_DIR)
        .join("community-plugins.json")
}

/// `{vault}/.cobblestone/plugins/{id}/data.json`
pub fn plugin_data_path(vault_root: &Path, plugin_id: &str) -> PathBuf {
    let id = to_cobblestone_plugin_id(plugin_id);
    vault_root
        .join(COBBLESTONE_DIR)
        .join("plugins")
        .join(id)
        .join("data.json")
}

/// Optional upstream settings file for **import only** (never write here from Cobblestone).
pub fn upstream_plugin_data_path(vault_root: &Path, upstream_plugin_id: &str) -> PathBuf {
    vault_root
        .join(UPSTREAM_CONFIG_DIR)
        .join("plugins")
        .join(upstream_plugin_id)
        .join("data.json")
}

/// Note path helpers: ensure `.md` suffix for vault-relative paths.
pub fn to_vault_md_path(note_id_or_path: &str) -> String {
    let p = note_id_or_path.trim_start_matches('/');
    if p.ends_with(".md") {
        p.to_string()
    } else {
        format!("{p}.md")
    }
}

/// Strip `.md` for Cobblestone note ids.
pub fn to_note_id(path: &str) -> String {
    let p = path.trim_start_matches('/');
    p.strip_suffix(".md").unwrap_or(p).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_plugin_and_command_ids() {
        assert_eq!(to_cobblestone_plugin_id("obsidian-tasks"), "tasks");
        assert_eq!(to_cobblestone_plugin_id("tasks"), "tasks");
        assert_eq!(
            to_cobblestone_command_id("obsidian-tasks:list"),
            "tasks:list"
        );
        assert_eq!(to_upstream_command_id("tasks:list"), "obsidian-tasks:list");
    }

    #[test]
    fn normalizes_enabled_list() {
        let mut ids = vec!["obsidian-tasks".into(), "tasks".into(), "quickadd".into()];
        normalize_enabled_plugin_ids(&mut ids);
        assert_eq!(ids, vec!["quick-capture".to_string(), "tasks".to_string()]);
    }

    #[test]
    fn note_path_roundtrip() {
        assert_eq!(to_vault_md_path("Daily"), "Daily.md");
        assert_eq!(to_note_id("folder/Daily.md"), "folder/Daily");
    }
}
