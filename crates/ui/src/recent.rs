// SPDX-License-Identifier: MIT

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::AppCtx;
use crate::util::remap_path;

const LIMIT: usize = 5;
const MAX_AGE: u64 = 7 * 24 * 60 * 60;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct OpenedNote {
    slug: String,
    opened_at: u64,
}

#[derive(Default)]
pub(crate) struct RecentNotes {
    key: String,
    entries: Vec<OpenedNote>,
}

impl RecentNotes {
    pub(crate) async fn load(vault_path: Option<String>) -> Self {
        let key = match vault_path {
            Some(path) => format!("cb-recent-notes:{path}"),
            None => "cb-recent-notes".into(),
        };
        let eval = document::eval(
            r#"const key = await dioxus.recv();
            try { return JSON.parse(localStorage.getItem(key)); } catch (_) { return null; }"#,
        );
        let _ = eval.send(&key);
        let mut entries = eval
            .join::<Option<Vec<OpenedNote>>>()
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let time = now();
        entries.retain(|note| is_recent(note.opened_at, time));
        entries.truncate(LIMIT);
        Self { key, entries }
    }

    pub(crate) fn slugs(&self) -> impl Iterator<Item = &str> {
        let time = now();
        self.entries
            .iter()
            .filter(move |note| is_recent(note.opened_at, time))
            .map(|note| note.slug.as_str())
    }
}

pub(crate) async fn record(mut ctx: AppCtx, slug: &str) {
    let time = now();
    {
        let mut history = ctx.recent_notes.write();
        history
            .entries
            .retain(|note| note.slug != slug && is_recent(note.opened_at, time));
        history.entries.insert(
            0,
            OpenedNote {
                slug: slug.into(),
                opened_at: time,
            },
        );
        history.entries.truncate(LIMIT);
    }
    persist(ctx).await;
}

pub(crate) async fn remap(ctx: AppCtx, from: &str, to: &str) {
    if from == to {
        return;
    }
    let mut entries = ctx.recent_notes.peek().entries.clone();
    for note in &mut entries {
        note.slug = remap_path(&note.slug, from, to);
    }
    replace_entries(ctx, entries).await;
}

pub(crate) async fn remove_missing(ctx: AppCtx) {
    let mut entries = ctx.recent_notes.peek().entries.clone();
    {
        let notes = ctx.notes.peek();
        entries.retain(|entry| notes.iter().any(|note| note.slug == entry.slug));
    }
    replace_entries(ctx, entries).await;
}

async fn replace_entries(mut ctx: AppCtx, entries: Vec<OpenedNote>) {
    if ctx.recent_notes.peek().entries != entries {
        ctx.recent_notes.write().entries = entries;
        persist(ctx).await;
    }
}

async fn persist(ctx: AppCtx) {
    let eval = document::eval(
        r#"const [key, entries] = await dioxus.recv();
        try { localStorage.setItem(key, JSON.stringify(entries)); } catch (_) {}
        return null;"#,
    );
    {
        let history = ctx.recent_notes.peek();
        let _ = eval.send((&history.key, &history.entries));
    }
    let _ = eval.join::<()>().await;
}

fn is_recent(opened_at: u64, now: u64) -> bool {
    now.saturating_sub(opened_at) < MAX_AGE
}

#[cfg(target_arch = "wasm32")]
fn now() -> u64 {
    (web_sys::js_sys::Date::now() / 1000.0) as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> u64 {
    std::time::UNIX_EPOCH
        .elapsed()
        .unwrap_or_default()
        .as_secs()
}
