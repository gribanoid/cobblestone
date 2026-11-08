// SPDX-License-Identifier: MIT
//! Periodic Notes — daily / weekly / monthly open-or-create (see repository `NOTICE`).

use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate};
use cobblestone_plugin_api::{App, Command, Plugin, PluginError, PluginManifest, PluginResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicSettings {
    pub daily_folder: String,
    pub daily_format: String,
    pub weekly_folder: String,
    pub weekly_format: String,
    pub monthly_folder: String,
    pub monthly_format: String,
    pub template: String,
}

impl Default for PeriodicSettings {
    fn default() -> Self {
        Self {
            daily_folder: String::new(),
            daily_format: "%Y-%m-%d".into(),
            weekly_folder: String::new(),
            weekly_format: "%G-W%V".into(),
            monthly_folder: String::new(),
            monthly_format: "%Y-%m".into(),
            template: String::new(),
        }
    }
}

pub struct PeriodicNotesPlugin {
    manifest: PluginManifest,
    settings: PeriodicSettings,
    app: Option<Arc<App>>,
}

impl PeriodicNotesPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "periodic-notes".into(),
                name: "Periodic Notes".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: "Daily, weekly, and monthly notes".into(),
            },
            settings: PeriodicSettings::default(),
            app: None,
        }
    }

    fn load_settings(app: &App) -> PeriodicSettings {
        app.load_data("periodic-notes")
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default()
    }

    fn note_path(folder: &str, name: &str) -> String {
        let name = if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{name}.md")
        };
        if folder.is_empty() {
            name
        } else {
            format!("{}/{name}", folder.trim_matches('/'))
        }
    }

    fn open_or_create(app: &App, folder: &str, stem: &str, template: &str) -> PluginResult<()> {
        let path = Self::note_path(folder, stem);
        if !app.vault.exists(&path) {
            let title = stem.trim_end_matches(".md");
            let body = if template.is_empty() {
                format!("# {title}\n\n")
            } else if let Ok(tpl) = app.vault.read(template) {
                tpl.replace("{{title}}", title).replace("{{date}}", title)
            } else {
                format!("# {title}\n\n")
            };
            app.vault.create(&path, &body)?;
        }
        app.workspace.open_file(&path)?;
        (app.notice)(&format!("Opened {path}"));
        Ok(())
    }

    pub fn open_daily(app: &App, settings: &PeriodicSettings) -> PluginResult<()> {
        let stem = Local::now().format(&settings.daily_format).to_string();
        Self::open_or_create(app, &settings.daily_folder, &stem, &settings.template)
    }

    pub fn open_weekly(app: &App, settings: &PeriodicSettings) -> PluginResult<()> {
        let stem = Local::now().format(&settings.weekly_format).to_string();
        Self::open_or_create(app, &settings.weekly_folder, &stem, &settings.template)
    }

    pub fn open_monthly(app: &App, settings: &PeriodicSettings) -> PluginResult<()> {
        let stem = Local::now().format(&settings.monthly_format).to_string();
        Self::open_or_create(app, &settings.monthly_folder, &stem, &settings.template)
    }

    fn shift_active(app: &App, settings: &PeriodicSettings, days: i64) -> PluginResult<()> {
        let Some(active) = app.workspace.active_file_path() else {
            return Err(PluginError::msg("No active file"));
        };
        let stem = active
            .rsplit('/')
            .next()
            .unwrap_or(&active)
            .trim_end_matches(".md");
        if let Ok(date) = NaiveDate::parse_from_str(stem, &settings.daily_format) {
            let next = date + chrono::Duration::days(days);
            let new_stem = next.format(&settings.daily_format).to_string();
            return Self::open_or_create(
                app,
                &settings.daily_folder,
                &new_stem,
                &settings.template,
            );
        }
        Err(PluginError::msg(
            "Active note is not a daily note matching the configured format",
        ))
    }
}

impl Default for PeriodicNotesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for PeriodicNotesPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
        self.settings = Self::load_settings(&app);
        let settings = self.settings.clone();
        self.app = Some(Arc::clone(&app));

        let a = Arc::clone(&app);
        let s = settings.clone();
        app.commands.add(Command {
            id: "periodic-notes:open-daily".into(),
            name: "Open daily note".into(),
            callback: Arc::new(move || Self::open_daily(&a, &s)),
        });

        let a = Arc::clone(&app);
        let s = settings.clone();
        app.commands.add(Command {
            id: "periodic-notes:open-weekly".into(),
            name: "Open weekly note".into(),
            callback: Arc::new(move || Self::open_weekly(&a, &s)),
        });

        let a = Arc::clone(&app);
        let s = settings.clone();
        app.commands.add(Command {
            id: "periodic-notes:open-monthly".into(),
            name: "Open monthly note".into(),
            callback: Arc::new(move || Self::open_monthly(&a, &s)),
        });

        let a = Arc::clone(&app);
        let s = settings.clone();
        app.commands.add(Command {
            id: "periodic-notes:next".into(),
            name: "Next periodic note".into(),
            callback: Arc::new(move || Self::shift_active(&a, &s, 1)),
        });

        let a = Arc::clone(&app);
        let s = settings;
        app.commands.add(Command {
            id: "periodic-notes:prev".into(),
            name: "Previous periodic note".into(),
            callback: Arc::new(move || Self::shift_active(&a, &s, -1)),
        });

        Ok(())
    }

    fn onunload(&mut self) -> PluginResult<()> {
        self.app = None;
        Ok(())
    }
}

/// Resolve today's daily note path for CLI `cb today`.
pub fn today_note_path(settings: &PeriodicSettings) -> String {
    let stem = Local::now().format(&settings.daily_format).to_string();
    PeriodicNotesPlugin::note_path(&settings.daily_folder, &stem)
}

#[allow(dead_code)]
fn _keep_datelike(d: NaiveDate) -> u32 {
    d.day()
}
