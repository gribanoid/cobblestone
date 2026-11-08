// SPDX-License-Identifier: MIT

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Local;
use cobblestone_core::Store;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

pub fn cmd_new(store: &Store, title: &str) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        bail!("Note title cannot be empty.");
    }
    let name = store.note_id_from_title(None, title)?;
    if store.exists(&name) {
        bail!(
            "Note '{}' already exists. Use `cb edit {}` to edit it.",
            name,
            name
        );
    }
    let content = format!("# {title}\n\n");
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

/// Create a zip archive of the entire vault.
///
/// Default destination: `~/Documents/CobblestoneVault-backups/CobblestoneVault-<timestamp>.zip`.
/// Pass `output` to choose a file path, or a directory (the zip is placed inside it).
pub fn cmd_backup(store: &Store, output: Option<PathBuf>) -> Result<()> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let file_name = format!("CobblestoneVault-{stamp}.zip");

    let out_path = match output {
        Some(path) if path.is_dir() || path.to_string_lossy().ends_with('/') => {
            path.join(&file_name)
        }
        Some(path) => path,
        None => {
            let home = dirs::home_dir().context("Cannot locate home directory")?;
            home.join("Documents")
                .join("CobblestoneVault-backups")
                .join(&file_name)
        }
    };

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create backup directory '{}'", parent.display()))?;
    }

    let file = File::create(&out_path)
        .with_context(|| format!("Cannot create backup file '{}'", out_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let skip = fs::canonicalize(&out_path).ok();
    let mut file_count = 0usize;
    zip_dir_recursive(
        &mut zip,
        &store.root,
        &store.root,
        options,
        skip.as_deref(),
        &mut file_count,
    )?;
    zip.finish()
        .with_context(|| format!("Failed to finalize backup '{}'", out_path.display()))?;

    println!(
        "Backup written: {} ({} files from {})",
        out_path.display(),
        file_count,
        store.root.display()
    );
    Ok(())
}

fn zip_dir_recursive(
    zip: &mut zip::ZipWriter<File>,
    vault_root: &Path,
    dir: &Path,
    options: SimpleFileOptions,
    skip: Option<&Path>,
    file_count: &mut usize,
) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("Cannot read '{}'", dir.display()))?
        .collect::<std::result::Result<_, _>>()
        .with_context(|| format!("Cannot read '{}'", dir.display()))?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if let Some(skip) = skip
            && fs::canonicalize(&path).ok().as_deref() == Some(skip)
        {
            continue;
        }
        let rel = path
            .strip_prefix(vault_root)
            .with_context(|| format!("Path {} escaped vault root", path.display()))?;
        let name = rel.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(format!("{name}/"), options)
                .with_context(|| format!("Cannot add directory '{name}' to backup"))?;
            zip_dir_recursive(zip, vault_root, &path, options, skip, file_count)?;
        } else if path.is_file() {
            zip.start_file(&name, options)
                .with_context(|| format!("Cannot add file '{name}' to backup"))?;
            let mut src =
                File::open(&path).with_context(|| format!("Cannot open '{}'", path.display()))?;
            io::copy(&mut src, zip)
                .with_context(|| format!("Cannot write '{name}' into backup"))?;
            *file_count += 1;
        }
    }
    Ok(())
}

/// Restore vault files from a backup zip created by [`cmd_backup`].
///
/// Existing files conflict by default and abort the restore. Pass `force` (`-f`)
/// to overwrite them. Files present in the vault but absent from the archive are
/// left untouched.
pub fn cmd_restore(store: &Store, archive: &Path, force: bool) -> Result<()> {
    let file = File::open(archive)
        .with_context(|| format!("Cannot open backup '{}'", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .with_context(|| format!("'{}' is not a valid zip archive", archive.display()))?;

    let mut planned: Vec<(usize, PathBuf, String)> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();

    for i in 0..zip.len() {
        let entry = zip
            .by_index(i)
            .with_context(|| format!("Cannot read entry {i} from '{}'", archive.display()))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .enclosed_name()
            .map(|p| p.to_path_buf())
            .with_context(|| format!("Refusing unsafe path in backup: '{}'", entry.name()))?;
        validate_restore_relpath(&name)?;
        let dest = store.root.join(&name);
        ensure_within_vault(&store.root, &dest)?;

        let label = name.to_string_lossy().replace('\\', "/");
        if dest.exists() && !force {
            conflicts.push(label);
        } else {
            planned.push((i, dest, label));
        }
    }

    if !conflicts.is_empty() {
        conflicts.sort();
        let preview: Vec<_> = conflicts
            .iter()
            .take(10)
            .map(|s| format!("  - {s}"))
            .collect();
        let extra = conflicts.len().saturating_sub(preview.len());
        let mut msg = format!(
            "Restore aborted: {} existing file(s) would be overwritten.\n{}",
            conflicts.len(),
            preview.join("\n")
        );
        if extra > 0 {
            msg.push_str(&format!("\n  … and {extra} more"));
        }
        msg.push_str("\nRe-run with `-f` to overwrite.");
        bail!("{msg}");
    }

    let mut restored = 0usize;
    let mut overwritten = 0usize;
    for (index, dest, label) in planned {
        let mut entry = zip
            .by_index(index)
            .with_context(|| format!("Cannot read '{label}' from backup"))?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create '{}'", parent.display()))?;
        }
        let existed = dest.exists();
        let mut out =
            File::create(&dest).with_context(|| format!("Cannot write '{}'", dest.display()))?;
        io::copy(&mut entry, &mut out).with_context(|| format!("Cannot extract '{label}'"))?;
        restored += 1;
        if existed {
            overwritten += 1;
        }
    }

    if force && overwritten > 0 {
        println!(
            "Restored {restored} file(s) from {} into {} ({overwritten} overwritten).",
            archive.display(),
            store.root.display()
        );
    } else {
        println!(
            "Restored {restored} file(s) from {} into {}.",
            archive.display(),
            store.root.display()
        );
    }
    Ok(())
}

fn validate_restore_relpath(rel: &Path) -> Result<()> {
    if rel.as_os_str().is_empty() {
        bail!("Backup contains an empty path");
    }
    for component in rel.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            _ => bail!("Refusing unsafe path in backup: '{}'", rel.display()),
        }
    }
    Ok(())
}

fn ensure_within_vault(root: &Path, dest: &Path) -> Result<()> {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let parent = dest.parent().unwrap_or(dest);
    if parent.exists() {
        let parent = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
        if !parent.starts_with(&root) {
            bail!("Refusing to write outside vault: {}", dest.display());
        }
    }
    Ok(())
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
        } else if let Some(t) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            println!("  \x1b[34m•\x1b[0m  {t}");
        } else if let Some(t) = line.strip_prefix("> ") {
            println!("  \x1b[90m│\x1b[0m  \x1b[3m{t}\x1b[0m");
        } else {
            println!("{line}");
        }
    }
}

// ---------------------------------------------------------------------------
// Plugins / projects / today / omni
// ---------------------------------------------------------------------------

pub fn cmd_today(store: &Store) -> Result<()> {
    let host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    host.execute_command("periodic-notes:open-daily")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for path in host.workspace.take_open_requests() {
        let id = path.trim_end_matches(".md");
        println!("Today: {}", store.path_for(id).display());
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        std::process::Command::new(&editor)
            .arg(store.path_for(id))
            .status()
            .with_context(|| format!("Failed to launch editor '{editor}'"))?;
    }
    Ok(())
}

pub fn cmd_plugin_list(store: &Store) -> Result<()> {
    let host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for (m, on) in host.list_plugins() {
        let flag = if on { "on " } else { "off" };
        println!("{flag}  {}  {}  v{}", m.id, m.name, m.version);
    }
    Ok(())
}

pub fn cmd_plugin_enable(store: &Store, id: &str, on: bool) -> Result<()> {
    let mut host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    host.set_enabled(id, on)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{} plugin '{id}' (restart commands to reload)",
        if on { "Enabled" } else { "Disabled" }
    );
    Ok(())
}

pub fn cmd_plugin_commands(store: &Store) -> Result<()> {
    let host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut cmds = host.list_commands();
    cmds.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, name) in cmds {
        println!("{id}\t{name}");
    }
    Ok(())
}

pub fn cmd_plugin_run(store: &Store, id: &str) -> Result<()> {
    let host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    host.execute_command(id)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for path in host.workspace.take_open_requests() {
        println!("open: {path}");
    }
    for notice in host.take_notices() {
        println!("{notice}");
    }
    Ok(())
}

pub fn cmd_project_list() -> Result<()> {
    let cfg = cobblestone_core::ProjectsConfig::load()?;
    if cfg.projects.is_empty() {
        println!("No projects registered. Use `cb project add <name> <path>`.");
        return Ok(());
    }
    for p in &cfg.projects {
        let mark = if cfg.last_opened.as_deref() == Some(&p.name) {
            "*"
        } else {
            " "
        };
        println!("{mark} {}  {}", p.name, p.path);
    }
    Ok(())
}

pub fn cmd_project_add(name: &str, path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    let mut cfg = cobblestone_core::ProjectsConfig::load()?;
    cfg.add(name, path)?;
    cfg.save()?;
    println!("Added project '{name}' -> {}", path.display());
    Ok(())
}

pub fn cmd_project_open(name: &str) -> Result<()> {
    let mut cfg = cobblestone_core::ProjectsConfig::load()?;
    let entry = cfg
        .get(name)
        .with_context(|| format!("Unknown project '{name}'"))?
        .clone();
    if !Path::new(&entry.path).is_dir() {
        bail!("Project '{}' is unavailable: {}", entry.name, entry.path);
    }
    cfg.set_last_opened(name);
    cfg.save()?;
    println!("Opened project '{}' ({})", entry.name, entry.path);
    println!("Subsequent `cb` commands will use this vault (unless CB_DIR is set).");
    Ok(())
}

pub fn cmd_project_rm(name: &str) -> Result<()> {
    let mut cfg = cobblestone_core::ProjectsConfig::load()?;
    if !cfg.remove(name) {
        bail!("Unknown project '{name}'");
    }
    cfg.save()?;
    println!("Removed project '{name}'");
    Ok(())
}

pub fn cmd_omni(store: &Store, query: &str, limit: usize) -> Result<()> {
    let host = cobblestone_plugin_host::bootstrap_builtins(clone_store(store)?, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let hits = cobblestone_plugin_omnisearch::search_vault(&host.app, query, limit)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if hits.is_empty() {
        println!("No matches.");
        return Ok(());
    }
    for h in hits {
        println!("{:.3}\t{}\t{}", h.score, h.path, h.snippet);
    }
    Ok(())
}

/// `Store` is not Clone — rebuild from the same root for the plugin host.
fn clone_store(store: &Store) -> Result<Store> {
    Store::with_root(store.root.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_creates_zip_with_notes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::with_root(dir.path().to_path_buf()).expect("store");
        store.write("hello", "# Hello\n").expect("write");
        store.create_folder("ideas").expect("folder");
        store
            .write("ideas/nested", "# Nested\n")
            .expect("write nested");

        let out = dir.path().join("out").join("vault.zip");
        cmd_backup(&store, Some(out.clone())).expect("backup");
        assert!(out.is_file());
        assert!(out.metadata().expect("meta").len() > 0);
    }

    #[test]
    fn restore_into_empty_vault() {
        let src = tempfile::tempdir().expect("src");
        let dst = tempfile::tempdir().expect("dst");
        let store = Store::with_root(src.path().to_path_buf()).expect("store");
        store.write("hello", "# Hello\n").expect("write");
        let zip_path = src.path().join("backup.zip");
        cmd_backup(&store, Some(zip_path.clone())).expect("backup");

        let target = Store::with_root(dst.path().to_path_buf()).expect("target");
        cmd_restore(&target, &zip_path, false).expect("restore");
        assert_eq!(target.read("hello").expect("read"), "# Hello\n");
    }

    #[test]
    fn restore_conflicts_without_force() {
        let src = tempfile::tempdir().expect("src");
        let dst = tempfile::tempdir().expect("dst");
        let store = Store::with_root(src.path().to_path_buf()).expect("store");
        store.write("hello", "# From backup\n").expect("write");
        let zip_path = src.path().join("backup.zip");
        cmd_backup(&store, Some(zip_path.clone())).expect("backup");

        let target = Store::with_root(dst.path().to_path_buf()).expect("target");
        target.write("hello", "# Existing\n").expect("existing");
        let err = cmd_restore(&target, &zip_path, false).expect_err("conflict");
        assert!(err.to_string().contains("-f"));
        assert_eq!(target.read("hello").expect("read"), "# Existing\n");
    }

    #[test]
    fn restore_force_overwrites() {
        let src = tempfile::tempdir().expect("src");
        let dst = tempfile::tempdir().expect("dst");
        let store = Store::with_root(src.path().to_path_buf()).expect("store");
        store.write("hello", "# From backup\n").expect("write");
        let zip_path = src.path().join("backup.zip");
        cmd_backup(&store, Some(zip_path.clone())).expect("backup");

        let target = Store::with_root(dst.path().to_path_buf()).expect("target");
        target.write("hello", "# Existing\n").expect("existing");
        cmd_restore(&target, &zip_path, true).expect("force restore");
        assert_eq!(target.read("hello").expect("read"), "# From backup\n");
    }

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
