// SPDX-License-Identifier: MIT
//! Omni search — ranked full-text vault search (see repository `NOTICE`).

use std::collections::HashMap;
use std::sync::Arc;

use cobblestone_plugin_api::{App, Command, Plugin, PluginManifest, PluginResult};

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

/// Tokenize for search: whitespace + camelCase / snake_case splits.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for raw in text.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if raw.is_empty() {
            continue;
        }
        tokens.push(raw.to_lowercase());
        for part in split_ident(raw) {
            let p = part.to_lowercase();
            if p.len() > 1 && !tokens.contains(&p) {
                tokens.push(p);
            }
        }
    }
    tokens
}

fn split_ident(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    if s.contains('_') {
        out.extend(s.split('_').filter(|p| !p.is_empty()).map(str::to_string));
    }
    let mut cur = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            if !cur.is_empty() {
                out.push(cur.clone());
            }
            cur.clear();
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[derive(Default)]
pub struct SearchIndex {
    /// term -> (path -> tf)
    postings: HashMap<String, HashMap<String, u32>>,
    docs: HashMap<String, String>,
    doc_len: HashMap<String, u32>,
    avg_len: f32,
}

impl SearchIndex {
    pub fn build(app: &App) -> PluginResult<Self> {
        let mut idx = Self::default();
        let files = app.vault.list_markdown_files()?;
        let mut total_len = 0u32;
        for file in files {
            let content = app.vault.read(&file.path).unwrap_or_default();
            let mut text = file.basename.clone();
            text.push('\n');
            text.push_str(&content);
            if let Some(meta) = app.metadata_cache.get_file_cache(&file.path) {
                for h in meta.headings {
                    text.push('\n');
                    text.push_str(&h);
                }
            }
            let tokens = tokenize(&text);
            let len = tokens.len() as u32;
            total_len += len;
            idx.doc_len.insert(file.path.clone(), len.max(1));
            idx.docs.insert(file.path.clone(), content);
            let mut tf: HashMap<String, u32> = HashMap::new();
            for t in tokens {
                *tf.entry(t).or_default() += 1;
            }
            for (term, count) in tf {
                idx.postings
                    .entry(term)
                    .or_default()
                    .insert(file.path.clone(), count);
            }
        }
        let n = idx.docs.len().max(1) as f32;
        idx.avg_len = (total_len as f32) / n;
        Ok(idx)
    }

    /// BM25-like ranking (k1=1.2, b=0.75).
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let terms = tokenize(query);
        if terms.is_empty() {
            return Vec::new();
        }
        let n = self.docs.len().max(1) as f32;
        let k1 = 1.2f32;
        let b = 0.75f32;
        let mut scores: HashMap<String, f32> = HashMap::new();

        for term in &terms {
            let Some(posting) = self.postings.get(term) else {
                continue;
            };
            let df = posting.len() as f32;
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for (path, &tf) in posting {
                let dl = *self.doc_len.get(path).unwrap_or(&1) as f32;
                let tf = tf as f32;
                let denom = tf + k1 * (1.0 - b + b * dl / self.avg_len.max(1.0));
                let score = idf * (tf * (k1 + 1.0)) / denom;
                *scores.entry(path.clone()).or_default() += score;
            }
        }

        let mut hits: Vec<_> = scores
            .into_iter()
            .map(|(path, score)| {
                let snippet = self
                    .docs
                    .get(&path)
                    .map(|c| {
                        c.lines()
                            .find(|l| {
                                let lower = l.to_lowercase();
                                terms.iter().any(|t| lower.contains(t))
                            })
                            .unwrap_or("")
                            .chars()
                            .take(120)
                            .collect()
                    })
                    .unwrap_or_default();
                SearchHit {
                    path,
                    score,
                    snippet,
                }
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);
        hits
    }
}

pub struct OmniSearchPlugin {
    manifest: PluginManifest,
}

impl OmniSearchPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "omni-search".into(),
                name: "Omni search".into(),
                version: "0.1.0".into(),
                min_app_version: "0.1.0".into(),
                description: "Ranked vault search".into(),
            },
        }
    }
}

impl Default for OmniSearchPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OmniSearchPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn onload(&mut self, app: Arc<App>) -> PluginResult<()> {
        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "omni-search:reindex".into(),
            name: "Omni search: Reindex".into(),
            callback: Arc::new(move || {
                let idx = SearchIndex::build(&a)?;
                (a.notice)(&format!("Omni search indexed {} notes", idx.docs.len()));
                Ok(())
            }),
        });

        let a = Arc::clone(&app);
        app.commands.add(Command {
            id: "omni-search:open".into(),
            name: "Omni search: Focus search".into(),
            callback: Arc::new(move || {
                a.workspace.emit_ui("focus-search");
                (a.notice)("Search focused");
                Ok(())
            }),
        });
        Ok(())
    }
}

/// Convenience for CLI / UI.
pub fn search_vault(app: &App, query: &str, limit: usize) -> PluginResult<Vec<SearchHit>> {
    let idx = SearchIndex::build(app)?;
    Ok(idx.search(query, limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_camel_and_snake() {
        let t = tokenize("parseHttpResponse get_user_id");
        assert!(t.iter().any(|x| x == "http"));
        assert!(t.iter().any(|x| x == "user"));
        assert!(t.iter().any(|x| x == "get_user_id") || t.iter().any(|x| x == "get"));
    }
}
