// SPDX-License-Identifier: GPL-3.0-or-later
//
// Cobblestone — open-source knowledge base for your private thoughts
// Copyright (C) 2026  Cobblestone Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

pub struct Store {
    pub root: PathBuf,
}

impl Store {
    /// Initialise the default store at `~/.cobblestone`.
    pub fn init() -> Result<Self> {
        let home = dirs::home_dir().context("Cannot locate home directory")?;
        Self::with_root(home.join(".cobblestone"))
    }

    /// Initialise a store at an arbitrary root (used in tests and desktop).
    pub fn with_root(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root)
            .with_context(|| format!("Cannot create storage directory: {}", root.display()))?;
        Ok(Self { root })
    }

    // -----------------------------------------------------------------------
    // Note CRUD
    // -----------------------------------------------------------------------

    pub fn list_notes(&self) -> Result<Vec<Note>> {
        let mut notes = Vec::new();
        for entry in fs::read_dir(&self.root)
            .with_context(|| format!("Cannot read storage directory: {}", self.root.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match Note::from_path(&path) {
                    Ok(note) => notes.push(note),
                    Err(e) => {
                        eprintln!("warning: skipping {}: {e}", path.display());
                    }
                }
            }
        }
        notes.sort_by(|a, b| b.modified_raw.cmp(&a.modified_raw));
        Ok(notes)
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let slug = slugify(name);
        bail_if_empty_slug(&slug, name)?;
        let path = self.root.join(format!("{slug}.md"));
        fs::read_to_string(&path)
            .with_context(|| format!("Note '{}' not found", slug))
    }

    pub fn write(&self, name: &str, content: &str) -> Result<()> {
        let slug = slugify(name);
        bail_if_empty_slug(&slug, name)?;
        let path = self.root.join(format!("{slug}.md"));
        fs::write(&path, content)
            .with_context(|| format!("Cannot write note '{}'", slug))
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let slug = slugify(name);
        bail_if_empty_slug(&slug, name)?;
        let path = self.root.join(format!("{slug}.md"));
        fs::remove_file(&path)
            .with_context(|| format!("Note '{}' not found", slug))
    }

    pub fn exists(&self, name: &str) -> bool {
        let slug = slugify(name);
        if slug.is_empty() { return false; }
        self.root.join(format!("{slug}.md")).exists()
    }

    /// Returns the on-disk path for `name` (name is slugified first).
    pub fn path_for(&self, name: &str) -> PathBuf {
        let slug = slugify(name);
        self.root.join(format!("{slug}.md"))
    }

    pub fn search(&self, query: &str) -> Result<Vec<(Note, Vec<String>)>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        let q = query.to_lowercase();
        let mut results = Vec::new();

        for note in self.list_notes()? {
            let content = fs::read_to_string(&note.path).unwrap_or_default();
            let matches: Vec<String> = content
                .lines()
                .filter(|l| l.to_lowercase().contains(&q))
                .map(|l| l.trim().to_string())
                .take(3)
                .collect();

            if note.title.to_lowercase().contains(&q)
                || note.name.to_lowercase().contains(&q)
                || !matches.is_empty()
            {
                results.push((note, matches));
            }
        }
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Note metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub name:         String,
    pub title:        String,
    /// Stored as a `String` so serialisation is cross-platform.
    pub path:         String,
    pub modified:     String,
    pub modified_raw: u64,
    pub size:         u64,
    pub preview:      String,
    pub tags:         Vec<String>,
}

impl Note {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        let meta = fs::metadata(path)
            .with_context(|| format!("Cannot stat {}", path.display()))?;
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| name.clone());

        let preview: String = content
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(120)
            .collect();

        let tags = extract_tags(&content);

        let modified_raw = meta
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0);

        let modified = meta
            .modified()
            .map(|t| {
                let dt: chrono::DateTime<Local> = t.into();
                dt.format("%Y-%m-%d %H:%M").to_string()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            title,
            path: path.to_string_lossy().into_owned(),
            modified,
            modified_raw,
            size: meta.len(),
            preview,
            tags,
        })
    }
}

// ---------------------------------------------------------------------------
// CLI helpers
// ---------------------------------------------------------------------------

pub fn cmd_new(store: &Store, title: &str) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        bail!("Note title cannot be empty.");
    }
    let name = slugify(title);
    if store.exists(&name) {
        bail!("Note '{}' already exists. Use `cb edit {}` to edit it.", name, name);
    }
    let date    = Local::now().format("%Y-%m-%d").to_string();
    let content = format!("# {title}\n\n*Created: {date}*\n\n");
    store.write(&name, &content)?;

    println!("Created: {}", store.path_for(&name).display());

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor)
        .arg(store.path_for(&name))
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;
    Ok(())
}

pub fn cmd_show(store: &Store, name: &str) -> Result<()> {
    let content = store.read(name)?;
    render_to_terminal(&content);
    Ok(())
}

pub fn cmd_edit(store: &Store, name: &str) -> Result<()> {
    if !store.exists(name) {
        bail!("Note '{}' does not exist. Use `cb new` to create it.", name);
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor)
        .arg(store.path_for(name))
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;
    Ok(())
}

pub fn cmd_delete(store: &Store, name: &str) -> Result<()> {
    if !store.exists(name) {
        bail!("Note '{}' does not exist.", name);
    }
    print!("Delete '{name}'? [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        store.delete(name)?;
        println!("Deleted.");
    } else {
        println!("Aborted.");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

/// Convert an arbitrary string into a URL/filename-safe slug.
///
/// Rules:
/// - lower-case
/// - alphanumeric characters and `-` are kept
/// - everything else becomes `-`
/// - leading/trailing/consecutive `-` are collapsed
///
/// Returns an empty string for inputs that contain no alphanumeric characters.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Extract `#tag` tokens from note content.
///
/// A token qualifies as a tag when it:
/// - starts with `#`
/// - has at least one character after `#`
/// - appears after a word boundary (not at the very start of a line preceded
///   by nothing, which would be a Markdown heading)
pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in content.lines() {
        // Skip Markdown headings (# / ## / ### etc.)
        if line.starts_with('#') {
            continue;
        }
        for word in line.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag = word.trim_start_matches('#').to_string();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }
    tags
}

/// Simple Markdown → ANSI terminal renderer.
pub fn render_to_terminal(content: &str) {
    for line in content.lines() {
        if let Some(h) = line.strip_prefix("# ") {
            println!("\x1b[1;33m{h}\x1b[0m");
        } else if let Some(h) = line.strip_prefix("## ") {
            println!("\x1b[1;36m{h}\x1b[0m");
        } else if let Some(h) = line.strip_prefix("### ") {
            println!("\x1b[1;32m{h}\x1b[0m");
        } else if let Some(t) = line.strip_prefix("- [ ] ") {
            println!("  \x1b[31m☐\x1b[0m  {t}");
        } else if let Some(t) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            println!("  \x1b[32m✓\x1b[0m  \x1b[9m{t}\x1b[0m");
        } else if let Some(t) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
        {
            println!("  \x1b[34m•\x1b[0m  {t}");
        } else if let Some(t) = line.strip_prefix("> ") {
            println!("  \x1b[90m│\x1b[0m  \x1b[3m{t}\x1b[0m");
        } else {
            println!("{line}");
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn bail_if_empty_slug(slug: &str, original: &str) -> Result<()> {
    if slug.is_empty() {
        bail!("Note name '{}' produces an empty slug — use alphanumeric characters.", original);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Helpers ────────────────────────────────────────────────────────────

    fn temp_store() -> (TempDir, Store) {
        let dir = TempDir::new().expect("tempdir");
        let store = Store::with_root(dir.path().to_path_buf()).expect("store");
        (dir, store)
    }

    // ── slugify ────────────────────────────────────────────────────────────

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn slugify_empty_string() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn slugify_already_a_slug() {
        assert_eq!(slugify("hello-world"), "hello-world");
    }

    #[test]
    fn slugify_numbers() {
        assert_eq!(slugify("Note 123"), "note-123");
    }

    #[test]
    fn slugify_consecutive_spaces() {
        assert_eq!(slugify("a   b"), "a-b");
    }

    #[test]
    fn slugify_leading_trailing_dashes() {
        assert_eq!(slugify("  hello  "), "hello");
    }

    #[test]
    fn slugify_only_special_chars() {
        assert_eq!(slugify("!!!"), "");
    }

    #[test]
    fn slugify_mixed_case() {
        assert_eq!(slugify("MyCamelCase"), "mycamelcase");
    }

    #[test]
    fn slugify_hyphen_preserved() {
        assert_eq!(slugify("my-note"), "my-note");
    }

    // ── extract_tags ───────────────────────────────────────────────────────

    #[test]
    fn tags_basic() {
        let tags = extract_tags("Hello #rust #programming world");
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
    }

    #[test]
    fn tags_empty_content() {
        assert!(extract_tags("No tags here").is_empty());
    }

    #[test]
    fn tags_heading_not_a_tag() {
        // Lines that start with # are headings — should be skipped
        assert!(extract_tags("# My Heading").is_empty());
        assert!(extract_tags("## Section").is_empty());
    }

    #[test]
    fn tags_deduplication() {
        let tags = extract_tags("word #rust another #rust");
        assert_eq!(tags.iter().filter(|t| *t == "rust").count(), 1);
    }

    #[test]
    fn tags_multiline() {
        let content = "line one #alpha\nline two #beta\n# Heading #notag";
        let tags = extract_tags(content);
        assert!(tags.contains(&"alpha".to_string()));
        assert!(tags.contains(&"beta".to_string()));
        assert!(!tags.contains(&"notag".to_string()));
    }

    // ── Store::exists / write / read / delete ──────────────────────────────

    #[test]
    fn store_write_and_read_roundtrip() {
        let (_dir, store) = temp_store();
        store.write("test-note", "# Test\n\nContent").unwrap();
        assert_eq!(store.read("test-note").unwrap(), "# Test\n\nContent");
    }

    #[test]
    fn store_write_slugifies_name() {
        let (_dir, store) = temp_store();
        store.write("My Note", "content").unwrap();
        // "My Note" is slugified to "my-note" by the store
        assert!(store.exists("my-note"));
        // The file on disk must be "my-note.md", not "My Note.md"
        assert!(!store.root.join("My Note.md").exists());
    }

    #[test]
    fn store_exists_false_for_missing() {
        let (_dir, store) = temp_store();
        assert!(!store.exists("ghost"));
    }

    #[test]
    fn store_exists_true_after_write() {
        let (_dir, store) = temp_store();
        store.write("hello", "world").unwrap();
        assert!(store.exists("hello"));
    }

    #[test]
    fn store_delete_removes_file() {
        let (_dir, store) = temp_store();
        store.write("del-me", "bye").unwrap();
        assert!(store.exists("del-me"));
        store.delete("del-me").unwrap();
        assert!(!store.exists("del-me"));
    }

    #[test]
    fn store_delete_nonexistent_returns_err() {
        let (_dir, store) = temp_store();
        assert!(store.delete("ghost").is_err());
    }

    #[test]
    fn store_read_nonexistent_returns_err() {
        let (_dir, store) = temp_store();
        assert!(store.read("ghost").is_err());
    }

    #[test]
    fn store_write_empty_slug_returns_err() {
        let (_dir, store) = temp_store();
        assert!(store.write("!!!", "content").is_err());
    }

    #[test]
    fn store_read_empty_slug_returns_err() {
        let (_dir, store) = temp_store();
        assert!(store.read("!!!").is_err());
    }

    #[test]
    fn store_exists_empty_slug_returns_false() {
        let (_dir, store) = temp_store();
        assert!(!store.exists("!!!"));
    }

    // ── list_notes ─────────────────────────────────────────────────────────

    #[test]
    fn list_empty_store() {
        let (_dir, store) = temp_store();
        let notes = store.list_notes().unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn list_returns_all_md_files() {
        let (_dir, store) = temp_store();
        store.write("note-a", "# Note A").unwrap();
        store.write("note-b", "# Note B").unwrap();
        // A non-.md file should be ignored
        fs::write(store.root.join("other.txt"), "ignored").unwrap();
        let notes = store.list_notes().unwrap();
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn list_title_extracted_from_h1() {
        let (_dir, store) = temp_store();
        store.write("my-note", "# Great Title\n\nBody").unwrap();
        let notes = store.list_notes().unwrap();
        assert_eq!(notes[0].title, "Great Title");
    }

    #[test]
    fn list_title_falls_back_to_slug() {
        let (_dir, store) = temp_store();
        store.write("my-note", "No heading here").unwrap();
        let notes = store.list_notes().unwrap();
        assert_eq!(notes[0].title, "my-note");
    }

    #[test]
    fn list_preview_skips_headings() {
        let (_dir, store) = temp_store();
        store.write("p", "# Heading\n\nFirst real line").unwrap();
        let notes = store.list_notes().unwrap();
        assert!(notes[0].preview.contains("First real line"));
        assert!(!notes[0].preview.contains("Heading"));
    }

    #[test]
    fn list_tags_populated() {
        let (_dir, store) = temp_store();
        store.write("t", "# T\n\nContent #rust #coding").unwrap();
        let notes = store.list_notes().unwrap();
        assert!(notes[0].tags.contains(&"rust".to_string()));
        assert!(notes[0].tags.contains(&"coding".to_string()));
    }

    #[test]
    fn list_sorted_newest_first() {
        let (_dir, store) = temp_store();
        store.write("alpha", "# A").unwrap();
        store.write("beta",  "# B").unwrap();

        // Set alpha's mtime to epoch+1 s and beta's to epoch+2 s explicitly,
        // so this test is independent of OS/sandbox mtime resolution.
        let t_old = filetime::FileTime::from_unix_time(1, 0);
        let t_new = filetime::FileTime::from_unix_time(2, 0);
        filetime::set_file_mtime(store.path_for("alpha"), t_old).unwrap();
        filetime::set_file_mtime(store.path_for("beta"),  t_new).unwrap();

        let notes = store.list_notes().unwrap();
        assert_eq!(notes.len(), 2);
        assert_eq!(
            notes[0].name, "beta",
            "expected 'beta' (newer mtime) first; got '{}'",
            notes[0].name
        );
    }

    // ── search ─────────────────────────────────────────────────────────────

    #[test]
    fn search_finds_by_content() {
        let (_dir, store) = temp_store();
        store.write("note-a", "# Note A\n\nRust programming is fun").unwrap();
        store.write("note-b", "# Note B\n\nPython is dynamic").unwrap();
        let results = store.search("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.name, "note-a");
    }

    #[test]
    fn search_case_insensitive() {
        let (_dir, store) = temp_store();
        store.write("x", "# Rust Programming\n\ncontent").unwrap();
        assert_eq!(store.search("RUST").unwrap().len(), 1);
        assert_eq!(store.search("rust").unwrap().len(), 1);
        assert_eq!(store.search("Rust").unwrap().len(), 1);
    }

    #[test]
    fn search_by_title() {
        let (_dir, store) = temp_store();
        store.write("x", "# Unique Title Here\n\nbody").unwrap();
        let r = store.search("Unique Title").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn search_no_results() {
        let (_dir, store) = temp_store();
        store.write("x", "# Note\n\nsome content").unwrap();
        assert!(store.search("xyz123abc").unwrap().is_empty());
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let (_dir, store) = temp_store();
        store.write("x", "content").unwrap();
        assert!(store.search("").unwrap().is_empty());
        assert!(store.search("   ").unwrap().is_empty());
    }

    #[test]
    fn search_returns_matching_lines() {
        let (_dir, store) = temp_store();
        store.write("n", "# N\n\nfoo bar\nhello world\nbaz").unwrap();
        let results = store.search("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.iter().any(|l| l.contains("hello")));
    }

    // ── path_for ───────────────────────────────────────────────────────────

    #[test]
    fn path_for_ends_with_md() {
        let (_dir, store) = temp_store();
        let p = store.path_for("my note");
        assert!(p.to_str().unwrap().ends_with("my-note.md"));
    }

    // ── Note::from_path ────────────────────────────────────────────────────

    #[test]
    fn note_from_path_nonexistent_returns_err() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("ghost.md");
        assert!(Note::from_path(&path).is_err());
    }

    #[test]
    fn note_from_path_parses_correctly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("my-note.md");
        fs::write(&path, "# My Title\n\nPreview text #tag1").unwrap();
        let note = Note::from_path(&path).unwrap();
        assert_eq!(note.name, "my-note");
        assert_eq!(note.title, "My Title");
        assert!(note.preview.contains("Preview text"));
        assert!(note.tags.contains(&"tag1".to_string()));
        assert!(note.size > 0);
    }

    // ── render_to_terminal (smoke tests) ───────────────────────────────────

    #[test]
    fn render_does_not_panic_on_empty() {
        render_to_terminal("");
    }

    #[test]
    fn render_does_not_panic_on_all_variants() {
        let md = "# H1\n## H2\n### H3\n- [ ] todo\n- [x] done\n- item\n* star\n> quote\nplain";
        render_to_terminal(md);
    }
}
