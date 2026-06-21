// SPDX-License-Identifier: MIT

use std::fs;
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
        collect_notes_recursive(&self.root, &self.root, &mut notes)?;
        notes.sort_by(|a, b| b.modified_raw.cmp(&a.modified_raw));
        Ok(notes)
    }

    /// Hierarchical vault tree (folders + notes) for file-tree UIs.
    pub fn list_tree(&self) -> Result<Vec<VaultNode>> {
        read_tree_dir(&self.root, &self.root)
    }

    pub fn create_folder(&self, path: &str) -> Result<()> {
        let path = normalize_folder_path(path)?;
        let dir = self.root.join(&path);
        fs::create_dir_all(&dir)
            .with_context(|| format!("Cannot create folder '{}'", path))
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let id = resolve_note_id(name)?;
        let path = self.root.join(format!("{id}.md"));
        fs::read_to_string(&path)
            .with_context(|| format!("Note '{id}' not found"))
    }

    pub fn write(&self, name: &str, content: &str) -> Result<()> {
        let id = resolve_note_id(name)?;
        let path = self.root.join(format!("{id}.md"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create parent directory for '{id}'"))?;
        }
        fs::write(&path, content)
            .with_context(|| format!("Cannot write note '{id}'"))
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let id = resolve_note_id(name)?;
        let path = self.root.join(format!("{id}.md"));
        fs::remove_file(&path)
            .with_context(|| format!("Note '{id}' not found"))
    }

    /// Move a note into `dest_folder` (or vault root when `None`/empty).
    /// Returns the new note id.
    pub fn move_note(&self, from: &str, dest_folder: Option<&str>) -> Result<String> {
        let from_id = resolve_note_id(from)?;
        let from_path = self.root.join(format!("{from_id}.md"));
        if !from_path.is_file() {
            bail!("Note '{from_id}' not found");
        }

        let file_name = from_id
            .rsplit('/')
            .next()
            .context("Invalid note id")?;
        let new_id = match dest_folder {
            Some(folder) if !folder.trim().is_empty() => {
                let folder = normalize_folder_path(folder)?;
                format!("{folder}/{file_name}")
            }
            _ => file_name.to_string(),
        };

        if from_id == new_id {
            return Ok(new_id);
        }
        if self.exists(&new_id) {
            bail!("Note '{new_id}' already exists");
        }

        let to_path = self.root.join(format!("{new_id}.md"));
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create parent directory for '{new_id}'"))?;
        }
        fs::rename(&from_path, &to_path)
            .with_context(|| format!("Cannot move '{from_id}' to '{new_id}'"))?;
        Ok(new_id)
    }

    /// Move a folder into `dest_parent` (or vault root when `None`/empty).
    /// Returns the new folder path.
    pub fn move_folder(&self, from: &str, dest_parent: Option<&str>) -> Result<String> {
        let from = normalize_folder_path(from)?;
        let from_dir = self.root.join(&from);
        if !from_dir.is_dir() {
            bail!("Folder '{from}' not found");
        }

        let name = from
            .rsplit('/')
            .next()
            .context("Invalid folder path")?;
        let new_path = match dest_parent {
            Some(parent) if !parent.trim().is_empty() => {
                let parent = normalize_folder_path(parent)?;
                if parent == from || parent.starts_with(&format!("{from}/")) {
                    bail!("Cannot move folder into itself or a subfolder");
                }
                format!("{parent}/{name}")
            }
            _ => name.to_string(),
        };

        if from == new_path {
            return Ok(new_path);
        }

        let to_dir = self.root.join(&new_path);
        if to_dir.exists() {
            bail!("Folder '{new_path}' already exists");
        }

        if let Some(parent) = to_dir.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create parent directory for '{new_path}'"))?;
        }

        fs::rename(&from_dir, &to_dir)
            .with_context(|| format!("Cannot move folder '{from}' to '{new_path}'"))?;
        Ok(new_path)
    }

    /// Rename a note (updates file id from title and `#` heading in content).
    pub fn rename_note(&self, from: &str, new_title: &str) -> Result<String> {
        let from_id = resolve_note_id(from)?;
        let new_title = new_title.trim();
        if new_title.is_empty() {
            bail!("Note title cannot be empty");
        }
        let new_file = slugify(new_title);
        bail_if_empty_slug(&new_file, new_title)?;

        let new_id = match from_id.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/{new_file}"),
            None => new_file,
        };

        let from_path = self.root.join(format!("{from_id}.md"));
        if !from_path.is_file() {
            bail!("Note '{from_id}' not found");
        }

        let content = fs::read_to_string(&from_path)
            .with_context(|| format!("Cannot read note '{from_id}'"))?;
        let new_content = set_content_title(&content, new_title);

        if from_id == new_id {
            fs::write(&from_path, new_content)
                .with_context(|| format!("Cannot write note '{from_id}'"))?;
            return Ok(new_id);
        }

        if self.exists(&new_id) {
            bail!("Note '{new_id}' already exists");
        }

        let to_path = self.root.join(format!("{new_id}.md"));
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create parent directory for '{new_id}'"))?;
        }
        fs::write(&to_path, &new_content)
            .with_context(|| format!("Cannot write note '{new_id}'"))?;
        fs::remove_file(&from_path)
            .with_context(|| format!("Cannot remove note '{from_id}'"))?;
        Ok(new_id)
    }

    /// Rename a folder in place (same parent, new name). Returns the new path.
    pub fn rename_folder(&self, from: &str, new_name: &str) -> Result<String> {
        let from = normalize_folder_path(from)?;
        let new_name = new_name.trim();
        if new_name.is_empty() {
            bail!("Folder name cannot be empty");
        }
        validate_path_components(new_name)?;

        let new_path = match from.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/{new_name}"),
            None => new_name.to_string(),
        };

        if from == new_path {
            return Ok(new_path);
        }

        let from_dir = self.root.join(&from);
        if !from_dir.is_dir() {
            bail!("Folder '{from}' not found");
        }

        let to_dir = self.root.join(&new_path);
        if to_dir.exists() {
            bail!("Folder '{new_path}' already exists");
        }

        fs::rename(&from_dir, &to_dir)
            .with_context(|| format!("Cannot rename folder '{from}' to '{new_path}'"))?;
        Ok(new_path)
    }

    /// Delete a folder and all contents recursively.
    pub fn delete_folder(&self, path: &str) -> Result<()> {
        let path = normalize_folder_path(path)?;
        let dir = self.root.join(&path);
        if !dir.is_dir() {
            bail!("Folder '{path}' not found");
        }
        if dir == self.root {
            bail!("Cannot delete vault root");
        }
        fs::remove_dir_all(&dir)
            .with_context(|| format!("Cannot delete folder '{path}'"))?;
        Ok(())
    }

    pub fn exists(&self, name: &str) -> bool {
        self.note_file_path(name)
            .ok()
            .is_some_and(|path| path.exists())
    }

    /// Returns the on-disk path for `name` (relative note id).
    pub fn path_for(&self, name: &str) -> PathBuf {
        self.note_file_path(name).unwrap_or_else(|_| self.root.join(name))
    }

    /// Build a note id from an optional folder path and a human title.
    pub fn note_id_from_title(&self, folder: Option<&str>, title: &str) -> Result<String> {
        let file = slugify(title.trim());
        bail_if_empty_slug(&file, title)?;
        match folder {
            Some(folder) if !folder.trim().is_empty() => {
                let folder = normalize_folder_path(folder)?;
                Ok(format!("{folder}/{file}"))
            }
            _ => Ok(file),
        }
    }

    fn note_file_path(&self, name: &str) -> Result<PathBuf> {
        let id = resolve_note_id(name)?;
        Ok(self.root.join(format!("{id}.md")))
    }

    pub fn search(&self, query: &str) -> Result<Vec<(Note, Vec<String>)>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        let q = query.to_lowercase();
        let mut results = Vec::new();
        let mut paths = Vec::new();
        collect_md_paths_recursive(&self.root, &mut paths)?;

        for path in paths {
            let (note, content) = match Note::from_path_with_content(&self.root, &path) {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("warning: skipping {}: {e}", path.display());
                    continue;
                }
            };
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
        results.sort_by(|a, b| b.0.modified_raw.cmp(&a.0.modified_raw));
        Ok(results)
    }

    /// Outgoing wikilinks and backlinks for a note (for graph / info panels).
    pub fn note_graph(&self, slug: &str) -> Result<NoteGraph> {
        let id = resolve_note_id(slug)?;
        let notes = self.list_notes()?;
        let current = self.read(&id)?;

        let outgoing_titles = extract_wikilinks(&current);
        let outgoing = notes
            .iter()
            .filter(|note| {
                outgoing_titles.iter().any(|link| {
                    link.eq_ignore_ascii_case(&note.title) || link.eq_ignore_ascii_case(&note.name)
                })
            })
            .map(|note| LinkedNote {
                slug: note.name.clone(),
                title: note.title.clone(),
            })
            .collect();

        let current_title = notes
            .iter()
            .find(|note| note.name == id)
            .map(|note| note.title.clone())
            .unwrap_or_else(|| id.clone());
        let current_markers = [format!("[[{current_title}]]"), format!("[[{id}]]")];

        let backlinks = notes
            .iter()
            .filter(|note| note.name != id)
            .filter_map(|note| {
                let content = self.read(&note.name).ok()?;
                let has_backlink = current_markers
                    .iter()
                    .any(|marker| content.to_lowercase().contains(&marker.to_lowercase()));
                has_backlink.then(|| LinkedNote {
                    slug: note.name.clone(),
                    title: note.title.clone(),
                })
            })
            .collect();

        Ok(NoteGraph {
            outgoing,
            backlinks,
        })
    }
}

// ---------------------------------------------------------------------------
// Note graph (wikilinks)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedNote {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteGraph {
    pub outgoing: Vec<LinkedNote>,
    pub backlinks: Vec<LinkedNote>,
}

// ---------------------------------------------------------------------------
// Vault tree
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VaultNode {
    Folder {
        name: String,
        path: String,
        children: Vec<VaultNode>,
    },
    Note {
        slug: String,
        title: String,
        created: String,
        modified: String,
        size: u64,
        preview: String,
        tags: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Note metadata
// ---------------------------------------------------------------------------

fn format_file_time(t: std::time::SystemTime) -> String {
    let dt: chrono::DateTime<Local> = t.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn is_created_stamp(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("*Created:") && t.ends_with('*')
}

fn set_content_title(content: &str, title: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.first().is_some_and(|l| l.starts_with("# ")) {
        let rest = lines[1..].join("\n");
        if rest.is_empty() {
            format!("# {title}")
        } else {
            format!("# {title}\n{rest}")
        }
    } else {
        format!("# {title}\n\n{content}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub name:         String,
    pub title:        String,
    /// Stored as a `String` so serialisation is cross-platform.
    pub path:         String,
    pub created:      String,
    pub modified:     String,
    pub modified_raw: u64,
    pub size:         u64,
    pub preview:      String,
    pub tags:         Vec<String>,
}

impl Note {
    pub fn from_path(root: &Path, path: &Path) -> Result<Self> {
        let (note, _) = Self::from_path_with_content(root, path)?;
        Ok(note)
    }

    /// Like `from_path` but also returns the raw file content.
    fn from_path_with_content(root: &Path, path: &Path) -> Result<(Self, String)> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        let meta = fs::metadata(path)
            .with_context(|| format!("Cannot stat {}", path.display()))?;
        let name = relative_note_id(root, path)?;

        let title = content
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches("# ").trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                name.rsplit('/')
                    .next()
                    .unwrap_or(&name)
                    .to_string()
            });

        let preview: String = content
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty() && !is_created_stamp(l))
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(120)
            .collect();

        let tags = extract_tags(&content);

        let created_time = meta.created().ok().or_else(|| meta.modified().ok());
        let created = created_time
            .map(|t| format_file_time(t))
            .unwrap_or_default();

        let sys_time = meta.modified().ok();
        let modified_raw = sys_time
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let modified = sys_time
            .map(format_file_time)
            .unwrap_or_default();

        let note = Self {
            name,
            title,
            path: path.to_string_lossy().into_owned(),
            created,
            modified,
            modified_raw,
            size: meta.len(),
            preview,
            tags,
        };
        Ok((note, content))
    }
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
/// - has at least one alphanumeric character after `#`
/// - appears after a word boundary (not at the very start of a line,
///   which would be a Markdown heading)
///
/// Only alphanumeric characters, `-`, and `_` are included in the tag name —
/// trailing punctuation like `#rust,` yields the tag `rust`.
pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for line in content.lines() {
        // Skip Markdown headings (# / ## / ### etc.)
        if line.starts_with('#') {
            continue;
        }
        for word in line.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 {
                let tag: String = word
                    .trim_start_matches('#')
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }
    tags
}

/// Extract `[[wikilink]]` targets from note content (deduplicated, in order).
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = content;

    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };

        let title = rest[..end].trim();
        if !title.is_empty() && !links.iter().any(|existing: &String| existing == title) {
            links.push(title.to_string());
        }
        rest = &rest[end + 2..];
    }

    links
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn collect_md_paths_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Cannot read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_md_paths_recursive(&path, out)?;
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_notes_recursive(root: &Path, dir: &Path, out: &mut Vec<Note>) -> Result<()> {
    let mut paths = Vec::new();
    collect_md_paths_recursive(dir, &mut paths)?;
    for path in paths {
        match Note::from_path(root, &path) {
            Ok(note) => out.push(note),
            Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
        }
    }
    Ok(())
}

fn read_tree_dir(root: &Path, dir: &Path) -> Result<Vec<VaultNode>> {
    let mut folders = Vec::new();
    let mut notes = Vec::new();

    for entry in fs::read_dir(dir)
        .with_context(|| format!("Cannot read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = relative_note_id(root, &path)?;
            let children = read_tree_dir(root, &path)?;
            folders.push(VaultNode::Folder {
                name,
                path: rel,
                children,
            });
        } else if path.extension().map(|e| e == "md").unwrap_or(false) {
            match Note::from_path(root, &path) {
                Ok(note) => notes.push(VaultNode::Note {
                    slug: note.name,
                    title: note.title,
                    created: note.created,
                    modified: note.modified,
                    size: note.size,
                    preview: note.preview,
                    tags: note.tags,
                }),
                Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
            }
        }
    }

    folders.sort_by(|a, b| folder_name(a).cmp(folder_name(b)));
    notes.sort_by(|a, b| note_title(a).cmp(note_title(b)));
    Ok(folders.into_iter().chain(notes).collect())
}

fn folder_name(node: &VaultNode) -> &str {
    match node {
        VaultNode::Folder { name, .. } => name,
        VaultNode::Note { title, .. } => title,
    }
}

fn note_title(node: &VaultNode) -> &str {
    match node {
        VaultNode::Note { title, .. } => title,
        VaultNode::Folder { name, .. } => name,
    }
}

fn relative_note_id(root: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("Path {} is outside vault root", path.display()))?;
    let rel = rel.with_extension("");
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

/// Resolve a note id used in CRUD APIs.
/// Single-segment names are slugified; paths with `/` are validated as-is.
pub fn resolve_note_id(name: &str) -> Result<String> {
    let name = name.trim().replace('\\', "/");
    if name.is_empty() {
        bail!("Note id cannot be empty");
    }
    if name.starts_with('/') {
        bail!("Note id must be relative");
    }
    if name.contains('/') {
        validate_path_components(&name)?;
        Ok(name)
    } else {
        let slug = slugify(&name);
        bail_if_empty_slug(&slug, &name)?;
        Ok(slug)
    }
}

fn normalize_folder_path(path: &str) -> Result<String> {
    let path = path.trim().replace('\\', "/");
    if path.is_empty() {
        bail!("Folder path cannot be empty");
    }
    if path.starts_with('/') {
        bail!("Folder path must be relative");
    }
    validate_path_components(&path)?;
    Ok(path)
}

fn validate_path_components(path: &str) -> Result<()> {
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            bail!("Invalid path component in '{path}'");
        }
    }
    Ok(())
}

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
    fn tags_strip_trailing_punctuation() {
        // #rust, should yield "rust", not "rust,"
        let tags = extract_tags("Check out #rust, it's great");
        assert!(tags.contains(&"rust".to_string()), "got: {tags:?}");
        assert!(!tags.iter().any(|t| t.contains(',')));
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
        assert!(Note::from_path(dir.path(), &path).is_err());
    }

    #[test]
    fn note_from_path_parses_correctly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("my-note.md");
        fs::write(&path, "# My Title\n\nPreview text #tag1").unwrap();
        let note = Note::from_path(dir.path(), &path).unwrap();
        assert_eq!(note.name, "my-note");
        assert_eq!(note.title, "My Title");
        assert!(note.preview.contains("Preview text"));
        assert!(note.tags.contains(&"tag1".to_string()));
        assert!(note.size > 0);
    }

    #[test]
    fn nested_note_roundtrip() {
        let (_dir, store) = temp_store();
        store.write("projects/todo", "# Todo\n\nnested").unwrap();
        assert!(store.exists("projects/todo"));
        assert_eq!(store.read("projects/todo").unwrap(), "# Todo\n\nnested");
        let notes = store.list_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, "projects/todo");
    }

    #[test]
    fn list_tree_nested_folders() {
        let (_dir, store) = temp_store();
        store.create_folder("rust").unwrap();
        store.write("rust/intro", "# Intro").unwrap();
        store.write("readme", "# Readme").unwrap();

        let tree = store.list_tree().unwrap();
        assert_eq!(tree.len(), 2);

        let folder = tree.iter().find_map(|n| match n {
            VaultNode::Folder { name, children, .. } if name == "rust" => Some(children),
            _ => None,
        });
        assert!(folder.is_some());
        assert_eq!(folder.unwrap().len(), 1);
    }

    #[test]
    fn create_folder_idempotent() {
        let (_dir, store) = temp_store();
        store.create_folder("notes/daily").unwrap();
        store.create_folder("notes/daily").unwrap();
        assert!(store.root.join("notes/daily").is_dir());
    }

    #[test]
    fn move_note_into_folder() {
        let (_dir, store) = temp_store();
        store.create_folder("projects").unwrap();
        store.write("readme", "# Readme").unwrap();
        let new_id = store.move_note("readme", Some("projects")).unwrap();
        assert_eq!(new_id, "projects/readme");
        assert!(!store.exists("readme"));
        assert!(store.exists("projects/readme"));
    }

    #[test]
    fn move_note_to_root() {
        let (_dir, store) = temp_store();
        store.write("projects/todo", "# Todo").unwrap();
        let new_id = store.move_note("projects/todo", None).unwrap();
        assert_eq!(new_id, "todo");
        assert!(store.exists("todo"));
        assert!(!store.exists("projects/todo"));
    }

    #[test]
    fn move_folder_to_root() {
        let (_dir, store) = temp_store();
        store.create_folder("archive/old").unwrap();
        store.write("archive/old/note", "# N").unwrap();
        let new_path = store.move_folder("archive/old", None).unwrap();
        assert_eq!(new_path, "old");
        assert!(store.root.join("old").is_dir());
        assert!(store.exists("old/note"));
        assert!(!store.root.join("archive/old").exists());
    }

    #[test]
    fn move_folder_into_folder() {
        let (_dir, store) = temp_store();
        store.create_folder("src").unwrap();
        store.create_folder("docs").unwrap();
        store.write("src/readme", "# R").unwrap();
        let new_path = store.move_folder("src", Some("docs")).unwrap();
        assert_eq!(new_path, "docs/src");
        assert!(store.exists("docs/src/readme"));
        assert!(!store.root.join("src").exists());
    }

    #[test]
    fn move_folder_into_self_fails() {
        let (_dir, store) = temp_store();
        store.create_folder("a/b").unwrap();
        assert!(store.move_folder("a", Some("a/b")).is_err());
    }

    #[test]
    fn rename_note_updates_slug_and_title() {
        let (_dir, store) = temp_store();
        store.write("hello", "# Hello\n\nBody").unwrap();
        let new_id = store.rename_note("hello", "World").unwrap();
        assert_eq!(new_id, "world");
        assert!(!store.exists("hello"));
        let content = store.read("world").unwrap();
        assert!(content.starts_with("# World"));
    }

    #[test]
    fn rename_folder_in_place() {
        let (_dir, store) = temp_store();
        store.create_folder("old").unwrap();
        store.write("old/note", "# N").unwrap();
        let new_path = store.rename_folder("old", "new").unwrap();
        assert_eq!(new_path, "new");
        assert!(store.exists("new/note"));
        assert!(!store.root.join("old").exists());
    }

    #[test]
    fn delete_folder_removes_contents() {
        let (_dir, store) = temp_store();
        store.create_folder("tmp").unwrap();
        store.write("tmp/a", "# A").unwrap();
        store.delete_folder("tmp").unwrap();
        assert!(!store.root.join("tmp").exists());
    }

    // ── wikilinks / note graph ───────────────────────────────────────────────

    #[test]
    fn extract_wikilinks_dedupes() {
        let links = extract_wikilinks("See [[Foo]] and [[Foo]] again.");
        assert_eq!(links, vec!["Foo".to_string()]);
    }

    #[test]
    fn note_graph_outgoing_and_backlinks() {
        let (_dir, store) = temp_store();
        store.write("alpha", "# Alpha\n\nSee [[Beta]].").unwrap();
        store.write("beta", "# Beta\n\nLinks to [[Alpha]].").unwrap();
        store.write("lonely", "# Lonely\n\nNo links.").unwrap();

        let graph = store.note_graph("alpha").unwrap();
        assert_eq!(graph.outgoing.len(), 1);
        assert_eq!(graph.outgoing[0].slug, "beta");
        assert_eq!(graph.backlinks.len(), 1);
        assert_eq!(graph.backlinks[0].slug, "beta");
    }
}
