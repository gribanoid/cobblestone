// SPDX-License-Identifier: MIT
//! Dynamic Templates — dates, title, and cursor markers (no JS eval; see `NOTICE`).

use std::sync::Arc;

use chrono::Local;
use cobblestone_plugin_api::{App, Command, Plugin, PluginError, PluginManifest, PluginResult};
use serde::{Deserialize, Serialize};

/// Cursor marker inserted into Markdown; host editor may place caret here.
pub const CURSOR_MARKER: &str = "<% cursor %>";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatesSettings {
    #[serde(default = "default_folder")]
    pub folder: String,
}

fn default_folder() -> String {
    "templates".into()
}

impl Default for TemplatesSettings {
    fn default() -> Self {
        Self {
            folder: default_folder(),
        }
    }
}

/// Expand safe substitutions. No JS evaluation.
pub fn expand_template(src: &str, title: &str) -> String {
    let now = Local::now();
    src.replace("{{title}}", title)
        .replace("{{date}}", &now.format("%Y-%m-%d").to_string())
        .replace("{{time}}", &now.format("%H:%M").to_string())
        .replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M").to_string())
        .replace("tp.file.title", title)
        .replace("tp.date.now()", &now.format("%Y-%m-%d").to_string())
        .replace(CURSOR_MARKER, "") // strip for file write; UI may use separately
}

pub fn expand_keeping_cursor(src: &str, title: &str) -> (String, Option<usize>) {
    let now = Local::now();
    let mut s = src
        .replace("{{title}}", title)
        .replace("{{date}}", &now.format("%Y-%m-%d").to_string())
        .replace("{{time}}", &now.format("%H:%M").to_string())
        .replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M").to_string())
        .replace("tp.file.title", title)
        .replace("tp.date.now()", &now.format("%Y-%m-%d").to_string());
    let pos = s.find(CURSOR_MARKER).inspect(|_| {
        s = s.replacen(CURSOR_MARKER, "", 1);
    });
    (s, pos)
}

pub struct DynamicTemplatesPlugin {
    manifest: PluginManifest,
}

impl DynamicTemplatesPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "dynamic-templates".into(),
                name: "Dynamic Templates".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: "Dates, titles, and cursor markers in templates".into(),
            },
        }
    }

    fn settings(app: &App) -> TemplatesSettings {
        app.load_data("dynamic-templates")
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    pub fn create_from_template(app: &App, template_path: &str, title: &str) -> PluginResult<()> {
        let tpl = app.vault.read(template_path)?;
        let (body, _cursor) = expand_keeping_cursor(&tpl, title);
        let path = format!("{title}.md");
        if app.vault.exists(&path) {
            return Err(PluginError::msg(format!("Note already exists: {path}")));
        }
        app.vault.create(&path, &body)?;
        app.workspace.open_file(&path)?;
        (app.notice)(&format!("Created from template: {path}"));
        Ok(())
    }

    fn insert_default_template(app: &App) -> PluginResult<()> {
        let settings = Self::settings(app);
        let folder = settings.folder.trim_matches('/');
        let files = app.vault.list_markdown_files()?;
        let template = files
            .into_iter()
            .find(|f| f.path.starts_with(folder) || f.path.starts_with(&format!("{folder}/")));
        let Some(t) = template else {
            return Err(PluginError::msg(format!(
                "No templates found in folder '{folder}'"
            )));
        };
        let active = app
            .workspace
            .active_file_path()
            .ok_or_else(|| PluginError::msg("No active file"))?;
        let title = TFileBasename::from(&active);
        let tpl = app.vault.read(&t.path)?;
        let (fragment, _) = expand_keeping_cursor(&tpl, &title);
        // Prefer caret insert via host UI; fall back to append on disk.
        if app.editor.queue_insert(&fragment).is_ok() {
            (app.notice)("Template queued at caret");
        } else {
            app.editor.append_to_active(&fragment)?;
            (app.notice)("Template appended");
        }
        Ok(())
    }
}

struct TFileBasename;
impl TFileBasename {
    fn from(path: &str) -> String {
        path.rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".md")
            .to_string()
    }
}

impl Default for DynamicTemplatesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DynamicTemplatesPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "dynamic-templates:insert".into(),
            name: "Insert template".into(),
            callback: Arc::new(move || Self::insert_default_template(&a)),
        });

        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "dynamic-templates:new".into(),
            name: "New note from first template".into(),
            callback: Arc::new(move || {
                let settings = Self::settings(&a);
                let folder = settings.folder.trim_matches('/');
                let files = a.vault.list_markdown_files()?;
                let template = files
                    .into_iter()
                    .find(|f| f.path.starts_with(folder))
                    .ok_or_else(|| PluginError::msg("No template file found"))?;
                let default = format!("Untitled {}", Local::now().format("%Y%m%d-%H%M%S"));
                let title = (a.ask_value)("Note title", &default)?.unwrap_or(default);
                if title.trim().is_empty() {
                    return Err(PluginError::msg("Title required"));
                }
                Self::create_from_template(&a, &template.path, title.trim())
            }),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_and_finds_cursor() {
        let (out, pos) = expand_keeping_cursor("# {{title}}\n<% cursor %>\n", "Hello");
        assert!(out.starts_with("# Hello\n"));
        assert!(pos.is_some());
        assert!(!out.contains(CURSOR_MARKER));
    }
}
