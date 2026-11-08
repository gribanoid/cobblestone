// SPDX-License-Identifier: MIT
//! Tasks plugin — parse, query, toggle (see repository `NOTICE`).

use std::sync::Arc;

use chrono::{Days, NaiveDate};
use cobblestone_plugin_api::{
    App, Command, MarkdownPostProcessorContext, Plugin, PluginManifest, PluginResult,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskItem {
    pub path: String,
    pub line: usize,
    pub raw: String,
    pub description: String,
    pub done: bool,
    pub due: Option<String>,
    pub scheduled: Option<String>,
    pub repeat: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskQuery {
    pub status: Option<TaskStatusFilter>,
    pub path_contains: Option<String>,
    pub due_before: Option<String>,
    pub due_on: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatusFilter {
    Done,
    Todo,
}

pub fn parse_task_line(path: &str, line_no: usize, line: &str) -> Option<TaskItem> {
    let trimmed = line.trim_start();
    let (done, rest) = if let Some(r) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        (true, r)
    } else if let Some(r) = trimmed.strip_prefix("- [ ] ") {
        (false, r)
    } else {
        return None;
    };

    let due_re = Regex::new(r"📅\s*(\d{4}-\d{2}-\d{2})").ok()?;
    let sched_re = Regex::new(r"⏳\s*(\d{4}-\d{2}-\d{2})").ok()?;
    let repeat_re = Regex::new(r"🔁\s*(.+)").ok()?;

    let due = due_re
        .captures(rest)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
    let scheduled = sched_re
        .captures(rest)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));
    let repeat = repeat_re.captures(rest).map(|c| {
        c.get(1)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default()
    });

    let mut description = rest.to_string();
    for re in [&due_re, &sched_re, &repeat_re] {
        description = re.replace_all(&description, "").to_string();
    }
    description = description.trim().to_string();

    Some(TaskItem {
        path: path.to_string(),
        line: line_no,
        raw: line.to_string(),
        description,
        done,
        due,
        scheduled,
        repeat,
    })
}

pub fn parse_tasks_in_content(path: &str, content: &str) -> Vec<TaskItem> {
    content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| parse_task_line(path, i + 1, line))
        .collect()
}

pub fn parse_query_block(body: &str) -> TaskQuery {
    let mut q = TaskQuery::default();
    for line in body.lines() {
        let line = line.trim();
        if line == "done" {
            q.status = Some(TaskStatusFilter::Done);
        } else if line == "not done" || line == "todo" {
            q.status = Some(TaskStatusFilter::Todo);
        } else if let Some(p) = line.strip_prefix("path includes ") {
            q.path_contains = Some(p.trim().to_string());
        } else if let Some(d) = line.strip_prefix("due before ") {
            q.due_before = Some(d.trim().to_string());
        } else if let Some(d) = line.strip_prefix("due on ") {
            q.due_on = Some(d.trim().to_string());
        }
    }
    q
}

pub fn filter_tasks(tasks: &[TaskItem], query: &TaskQuery) -> Vec<TaskItem> {
    tasks
        .iter()
        .filter(|t| {
            if let Some(status) = query.status {
                match status {
                    TaskStatusFilter::Done if !t.done => return false,
                    TaskStatusFilter::Todo if t.done => return false,
                    _ => {}
                }
            }
            if let Some(ref p) = query.path_contains
                && !t.path.contains(p)
            {
                return false;
            }
            if let Some(ref due_on) = query.due_on
                && t.due.as_deref() != Some(due_on.as_str())
            {
                return false;
            }
            if let Some(ref before) = query.due_before {
                match (&t.due, NaiveDate::parse_from_str(before, "%Y-%m-%d")) {
                    (Some(d), Ok(limit)) => {
                        if let Ok(td) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                            if td >= limit {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            true
        })
        .cloned()
        .collect()
}

pub fn toggle_task_line(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("- [ ] ") {
        return Some(format!("- [x] {rest}"));
    }
    if let Some(rest) = line
        .strip_prefix("- [x] ")
        .or_else(|| line.strip_prefix("- [X] "))
    {
        return Some(format!("- [ ] {rest}"));
    }
    // preserve indent
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        return Some(format!("{indent}- [x] {rest}"));
    }
    if let Some(rest) = trimmed
        .strip_prefix("- [x] ")
        .or_else(|| trimmed.strip_prefix("- [X] "))
    {
        return Some(format!("{indent}- [ ] {rest}"));
    }
    None
}

pub fn render_tasks_html(tasks: &[TaskItem]) -> String {
    if tasks.is_empty() {
        return "<p><em>No matching tasks</em></p>".into();
    }
    let mut out = String::from("<ul class=\"cb-tasks-query\">");
    for t in tasks {
        let mark = if t.done { "☑" } else { "☐" };
        let due = t
            .due
            .as_ref()
            .map(|d| format!(" <span class=\"due\">📅 {d}</span>"))
            .unwrap_or_default();
        out.push_str(&format!(
            "<li>{mark} {}{} <small>({}:{})</small></li>",
            html_escape(&t.description),
            due,
            html_escape(&t.path),
            t.line
        ));
    }
    out.push_str("</ul>");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn process_tasks_blocks(
    html: &str,
    _ctx: &MarkdownPostProcessorContext,
    all: &[TaskItem],
) -> String {
    // Reading view: replace fenced ```tasks ... ``` that survived as <pre><code class="language-tasks">
    let re = Regex::new(r#"(?s)<pre><code class="language-tasks">(.*?)</code></pre>"#).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let body = html_unescape(&caps[1]);
        let q = parse_query_block(&body);
        let matched = filter_tasks(all, &q);
        render_tasks_html(&matched)
    })
    .into_owned()
}

fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

pub struct TasksPlugin {
    manifest: PluginManifest,
}

impl TasksPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "tasks".into(),
                name: "Tasks".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: "Task due dates, queries, and toggle".into(),
            },
        }
    }

    pub fn index_vault(app: &App) -> PluginResult<Vec<TaskItem>> {
        let mut all = Vec::new();
        for file in app.vault.list_markdown_files()? {
            let content = app.vault.read(&file.path)?;
            all.extend(parse_tasks_in_content(&file.path, &content));
        }
        Ok(all)
    }
}

impl Default for TasksPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for TasksPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
        let app_pp = Arc::clone(&app);
        app.register_markdown_post_processor(Arc::new(move |html, ctx| {
            if !html.contains(r#"<pre><code class="language-tasks">"#) {
                return html.to_owned();
            }
            let all = TasksPlugin::index_vault(&app_pp).unwrap_or_default();
            process_tasks_blocks(html, ctx, &all)
        }));

        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "tasks:list".into(),
            name: "List all tasks".into(),
            callback: Arc::new(move || {
                let tasks = TasksPlugin::index_vault(&a)?;
                let open = tasks.iter().filter(|t| !t.done).count();
                (a.notice)(&format!(
                    "{} tasks ({} open) — see right panel",
                    tasks.len(),
                    open
                ));
                a.workspace.emit_ui("show-tasks");
                Ok(())
            }),
        });

        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "tasks:toggle".into(),
            name: "Toggle task in active note".into(),
            callback: Arc::new(move || toggle_active_task(&a)),
        });

        Ok(())
    }
}

/// Toggle the first incomplete task in the active note (or the first task if all done).
pub fn toggle_active_task(app: &App) -> PluginResult<()> {
    let Some(path) = app.workspace.active_file_path() else {
        (app.notice)("No active note");
        return Ok(());
    };
    let content = app.vault.read(&path)?;
    let tasks = parse_tasks_in_content(&path, &content);
    if tasks.is_empty() {
        (app.notice)("No tasks in this note");
        return Ok(());
    }
    let target = tasks.iter().find(|t| !t.done).unwrap_or(&tasks[0]);
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let idx = target.line.saturating_sub(1);
    let Some(line) = lines.get(idx).cloned() else {
        return Ok(());
    };

    // Completing a recurring task: advance due, keep open.
    if !target.done
        && let Some(next_line) = apply_recurrence_on_complete(&line, target)
    {
        lines[idx] = next_line;
        write_lines(app, &path, &content, &lines)?;
        (app.notice)("Recurring task rescheduled");
        return Ok(());
    }

    let Some(next) = toggle_task_line(&line) else {
        (app.notice)("Could not toggle task line");
        return Ok(());
    };
    lines[idx] = next;
    write_lines(app, &path, &content, &lines)?;
    let now_done = parse_task_line(&path, target.line, &lines[idx])
        .map(|t| t.done)
        .unwrap_or(false);
    (app.notice)(if now_done {
        "Task completed"
    } else {
        "Task reopened"
    });
    Ok(())
}

fn write_lines(app: &App, path: &str, original: &str, lines: &[String]) -> PluginResult<()> {
    let mut new_content = lines.join("\n");
    if original.ends_with('\n') {
        new_content.push('\n');
    }
    app.vault.write(path, &new_content)
}

/// On complete: if `🔁 every day|week` and a due date exist, bump due and leave unchecked.
fn apply_recurrence_on_complete(line: &str, task: &TaskItem) -> Option<String> {
    let repeat = task.repeat.as_deref()?.to_ascii_lowercase();
    let due = task.due.as_deref()?;
    let base = NaiveDate::parse_from_str(due, "%Y-%m-%d").ok()?;
    let next = if repeat.contains("week") {
        base.checked_add_days(Days::new(7))?
    } else if repeat.contains("day") {
        base.checked_add_days(Days::new(1))?
    } else {
        return None;
    };
    let next_s = next.format("%Y-%m-%d").to_string();
    Some(line.replacen(due, &next_s, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_due_and_toggle() {
        let t = parse_task_line("a.md", 1, "- [ ] Buy milk 📅 2026-10-21 🔁 every day").unwrap();
        assert!(!t.done);
        assert_eq!(t.due.as_deref(), Some("2026-10-21"));
        assert_eq!(t.repeat.as_deref(), Some("every day"));
        assert_eq!(
            toggle_task_line("- [ ] Buy milk").as_deref(),
            Some("- [x] Buy milk")
        );
    }

    #[test]
    fn query_not_done() {
        let tasks = vec![
            parse_task_line("a.md", 1, "- [ ] one").unwrap(),
            parse_task_line("a.md", 2, "- [x] two").unwrap(),
        ];
        let q = parse_query_block("not done");
        let f = filter_tasks(&tasks, &q);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].description, "one");
    }
}
