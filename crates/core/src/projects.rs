// SPDX-License-Identifier: MIT
//! Multi-project (vault) registry — `~/.config/cobblestone/projects.toml`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectsConfig {
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    #[serde(default)]
    pub last_opened: Option<String>,
}

impl ProjectsConfig {
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Cannot locate home directory")?;
        Ok(home
            .join(".config")
            .join("cobblestone")
            .join("projects.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw =
            fs::read_to_string(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("Invalid TOML in {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let parent = path
            .parent()
            .context("Cannot locate project config directory")?;
        fs::create_dir_all(parent)?;
        let raw = toml::to_string_pretty(self)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(raw.as_bytes())?;
        file.persist(&path)?;
        Ok(())
    }

    pub fn add(&mut self, name: impl Into<String>, path: impl AsRef<Path>) -> Result<()> {
        let name = name.into().trim().to_owned();
        if name.is_empty() {
            bail!("Project name cannot be empty");
        }
        let path = path.as_ref().canonicalize()?;
        if !path.is_dir() {
            bail!("Project path is not a directory: {}", path.display());
        }
        let path_s = path
            .to_str()
            .context("Project path is not valid UTF-8")?
            .to_owned();
        if self
            .projects
            .iter()
            .any(|p| p.name == name || p.path == path_s)
        {
            bail!("Project already registered: {name}");
        }
        self.projects.push(ProjectEntry { name, path: path_s });
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.projects.len();
        self.projects.retain(|p| p.name != name);
        if self.last_opened.as_deref() == Some(name) {
            self.last_opened = None;
        }
        before != self.projects.len()
    }

    pub fn get(&self, name: &str) -> Option<&ProjectEntry> {
        self.projects.iter().find(|p| p.name == name)
    }

    pub fn set_last_opened(&mut self, name: &str) {
        self.last_opened = Some(name.to_string());
    }

    /// Ensure default vault is registered when config is empty.
    pub fn ensure_default(&mut self, default_root: &Path) -> Result<()> {
        if self.projects.is_empty() {
            let name = default_root
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "CobblestoneVault".into());
            fs::create_dir_all(default_root)?;
            self.add(name.clone(), default_root)?;
            self.set_last_opened(&name);
            self.save()?;
        }
        Ok(())
    }
}

/// Resolve vault root: `CB_DIR` > last_opened project > Store::default_vault_root.
pub fn resolve_vault_root() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("CB_DIR") {
        let p = PathBuf::from(dir);
        return Ok(p);
    }
    let mut cfg = ProjectsConfig::load()?;
    let default = crate::Store::default_vault_root()?;
    cfg.ensure_default(&default)?;
    if let Some(entry) = cfg.last_opened.as_deref().and_then(|name| cfg.get(name)) {
        let path = PathBuf::from(&entry.path);
        if !path.is_dir() {
            bail!(
                "Project '{}' is unavailable: {}",
                entry.name,
                path.display()
            );
        }
        return Ok(path);
    }
    Ok(default)
}
