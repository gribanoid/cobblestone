// SPDX-License-Identifier: MIT
//! Quick Capture — template / capture / macro / multi choices.
//! No arbitrary user JS or AI providers (see repository `NOTICE`).

use std::sync::Arc;

use chrono::Local;
use cobblestone_plugin_api::{App, Command, Plugin, PluginError, PluginManifest, PluginResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Choice {
    Template {
        name: String,
        template_path: String,
        #[serde(default)]
        folder: String,
        #[serde(default)]
        file_name_format: String,
    },
    Capture {
        name: String,
        target_path: String,
        #[serde(default)]
        format: String,
    },
    Macro {
        name: String,
        commands: Vec<String>,
    },
    Multi {
        name: String,
        choices: Vec<String>,
    },
}

impl Choice {
    pub fn name(&self) -> &str {
        match self {
            Choice::Template { name, .. }
            | Choice::Capture { name, .. }
            | Choice::Macro { name, .. }
            | Choice::Multi { name, .. } => name,
        }
    }

    pub fn needs_value(&self) -> bool {
        match self {
            Choice::Capture { format, .. } => format.contains("{{VALUE}}"),
            Choice::Template {
                file_name_format,
                template_path,
                ..
            } => {
                file_name_format.contains("{{VALUE}}")
                    || file_name_format.contains("{{NAME}}")
                    || !template_path.is_empty()
            }
            Choice::Multi { .. } => true,
            Choice::Macro { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuickCaptureData {
    #[serde(default)]
    pub choices: Vec<Choice>,
}

pub fn expand_format(
    fmt: &str,
    value: &str,
    name: &str,
    selection: &str,
    clipboard: &str,
) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    fmt.replace("{{DATE}}", &date)
        .replace("{{VALUE}}", value)
        .replace("{{NAME}}", name)
        .replace("{{SELECTION}}", selection)
        .replace("{{CLIPBOARD}}", clipboard)
}

pub struct QuickCapturePlugin {
    manifest: PluginManifest,
}

impl QuickCapturePlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "quick-capture".into(),
                name: "Quick Capture".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: "Fast capture and note creation".into(),
            },
        }
    }

    fn load_data(app: &App) -> QuickCaptureData {
        let mut data: QuickCaptureData = app
            .load_data("quick-capture")
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        if data.choices.is_empty() {
            data.choices.push(Choice::Template {
                name: "New from blank".into(),
                template_path: String::new(),
                folder: String::new(),
                file_name_format: "{{DATE}}".into(),
            });
            data.choices.push(Choice::Capture {
                name: "Capture to inbox".into(),
                target_path: "Inbox.md".into(),
                format: "- {{DATE}}: {{VALUE}}\n".into(),
            });
        }
        data
    }

    pub fn choice_needs_prompt(app: &App, index: usize) -> bool {
        Self::load_data(app)
            .choices
            .get(index)
            .map(|c| c.needs_value())
            .unwrap_or(false)
    }

    pub fn run_choice(app: &App, choice: &Choice, value: &str) -> PluginResult<()> {
        let selection = app.editor.selection();
        let clipboard = app.editor.clipboard();
        match choice {
            Choice::Template {
                template_path,
                folder,
                file_name_format,
                ..
            } => {
                let stem = expand_format(file_name_format, value, value, &selection, &clipboard);
                let stem = if stem.trim().is_empty() {
                    Local::now().format("%Y-%m-%d").to_string()
                } else {
                    stem
                };
                let path = if folder.is_empty() {
                    format!("{stem}.md")
                } else {
                    format!("{}/{stem}.md", folder.trim_matches('/'))
                };
                let body = if template_path.is_empty() {
                    format!("# {stem}\n\n{value}\n")
                } else {
                    let tpl = app.vault.read(template_path)?;
                    expand_format(&tpl, value, &stem, &selection, &clipboard)
                };
                app.vault.create(&path, &body)?;
                app.workspace.open_file(&path)?;
                (app.notice)(&format!("Created {path}"));
                Ok(())
            }
            Choice::Capture {
                target_path,
                format,
                ..
            } => {
                let line = expand_format(format, value, "", &selection, &clipboard);
                let mut content = if app.vault.exists(target_path) {
                    app.vault.read(target_path)?
                } else {
                    String::new()
                };
                if !content.ends_with('\n') && !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&line);
                if app.vault.exists(target_path) {
                    app.vault.write(target_path, &content)?;
                } else {
                    app.vault.create(target_path, &content)?;
                }
                (app.notice)(&format!("Captured to {target_path}"));
                Ok(())
            }
            Choice::Macro { commands, .. } => {
                for id in commands {
                    app.commands.execute(id)?;
                }
                Ok(())
            }
            Choice::Multi { name, choices } => {
                let pick = if value.is_empty() {
                    (app.ask_value)(
                        &format!("Pick choice for '{name}' ({})", choices.join(" | ")),
                        choices.first().map(|s| s.as_str()).unwrap_or(""),
                    )?
                    .unwrap_or_default()
                } else {
                    value.to_string()
                };
                let data = Self::load_data(app);
                let child = data
                    .choices
                    .iter()
                    .find(|c| c.name().eq_ignore_ascii_case(pick.trim()))
                    .ok_or_else(|| PluginError::msg(format!("Unknown choice '{pick}'")))?
                    .clone();
                if matches!(child, Choice::Multi { .. }) {
                    return Err(PluginError::msg("Nested Multi is not supported"));
                }
                Self::run_choice(app, &child, "")
            }
        }
    }

    fn run_indexed(app: &App, index: usize) -> PluginResult<()> {
        let data = Self::load_data(app);
        let choice = data
            .choices
            .get(index)
            .ok_or_else(|| PluginError::msg("Choice not found"))?
            .clone();
        let value = if choice.needs_value() {
            (app.ask_value)(&format!("Value for '{}'", choice.name()), "")?.unwrap_or_default()
        } else {
            String::new()
        };
        Self::run_choice(app, &choice, &value)
    }
}

impl Default for QuickCapturePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for QuickCapturePlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
        let data = Self::load_data(&app);
        for (i, choice) in data.choices.iter().enumerate() {
            let name = choice.name().to_string();
            let a = Arc::clone(&app);
            app.commands.add(Command {
                id: format!("quick-capture:choice-{i}"),
                name: format!("Quick Capture: {name}"),
                callback: Arc::new(move || Self::run_indexed(&a, i)),
            });
        }

        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "quick-capture:run".into(),
            name: "Quick Capture: Run first choice".into(),
            callback: Arc::new(move || Self::run_indexed(&a, 0)),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_placeholders() {
        let out = expand_format(
            "- {{DATE}} {{VALUE}} sel={{SELECTION}} clip={{CLIPBOARD}}",
            "buy milk",
            "n",
            "SEL",
            "CLIP",
        );
        assert!(out.contains("buy milk"));
        assert!(out.contains("sel=SEL"));
        assert!(out.contains("clip=CLIP"));
    }
}
