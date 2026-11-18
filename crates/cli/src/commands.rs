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
    store.create(&name, &content)?;

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

    if out_path.symlink_metadata().is_ok() {
        bail!(
            "Backup destination '{}' already exists; choose a new filename",
            out_path.display()
        );
    }
    if let Some(parent) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create backup directory '{}'", parent.display()))?;
    }

    let parent = out_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Cannot create backup file '{}'", out_path.display()))?;
    let mut zip = zip::ZipWriter::new(file.as_file().try_clone()?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let skip = fs::canonicalize(file.path()).ok();
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
    file.as_file().sync_all()?;
    file.persist_noclobber(&out_path).with_context(|| {
        format!(
            "Cannot publish backup '{}'; choose a new filename",
            out_path.display()
        )
    })?;

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
        if entry.file_type()?.is_symlink() {
            bail!(
                "Cannot back up symbolic link '{}'; store files inside the vault",
                path.display()
            );
        }
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

    let mut planned: Vec<(tempfile::NamedTempFile, PathBuf, String)> = Vec::new();
    let mut directories = Vec::new();
    let mut paths = std::collections::HashSet::new();
    let mut conflicts: Vec<String> = Vec::new();
    // Validate and decompress the entire archive before replacing any vault file.
    let staging = tempfile::tempdir_in(&store.root)?;
    let layout = staging.path().join("layout");
    fs::create_dir(&layout)?;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .with_context(|| format!("Cannot read entry {i} from '{}'", archive.display()))?;
        let name = entry
            .enclosed_name()
            .map(|p| p.to_path_buf())
            .with_context(|| format!("Refusing unsafe path in backup: '{}'", entry.name()))?;
        validate_restore_relpath(&name)?;
        if !paths.insert(name.clone()) {
            bail!("Duplicate path in backup: '{}'", name.display());
        }
        let dest = store.root.join(&name);
        ensure_within_vault(&store.root, &dest)?;
        // Test collisions on this volume, including case-insensitive APFS and
        // Unicode filename aliases, before publishing any restored file.
        let staged_path = layout.join(&name);
        if entry.is_dir() {
            if dest.exists() && !dest.is_dir() {
                bail!("Restore directory conflicts with file '{}'", dest.display());
            }
            fs::create_dir_all(&staged_path).with_context(|| {
                format!("Conflicting directory in backup: '{}'", name.display())
            })?;
            directories.push(dest);
            continue;
        }
        if dest.is_dir() {
            bail!("Restore file conflicts with directory '{}'", dest.display());
        }
        fs::create_dir_all(staged_path.parent().context("Backup entry has no parent")?)?;
        File::options()
            .write(true)
            .create_new(true)
            .open(&staged_path)
            .with_context(|| format!("Conflicting file in backup: '{}'", name.display()))?;

        let label = name.to_string_lossy().replace('\\', "/");
        if dest.exists() && !force {
            conflicts.push(label);
        } else {
            let mut staged = tempfile::NamedTempFile::new_in(staging.path())?;
            if dest.exists() {
                let permissions = fs::metadata(&dest)?.permissions();
                if permissions.readonly() {
                    bail!("Cannot overwrite read-only file '{}'", dest.display());
                }
                staged.as_file().set_permissions(permissions)?;
            }
            io::copy(&mut entry, staged.as_file_mut())
                .with_context(|| format!("Cannot extract '{label}'"))?;
            staged.as_file().sync_all()?;
            planned.push((staged, dest, label));
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
    for dir in directories {
        ensure_within_vault(&store.root, &dir)?;
        fs::create_dir_all(&dir)?;
    }
    for (staged, dest, label) in planned {
        ensure_within_vault(&store.root, &dest)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create '{}'", parent.display()))?;
        }
        let existed = dest.exists();
        if force {
            staged.persist(&dest)
        } else {
            staged.persist_noclobber(&dest)
        }
        .with_context(|| format!("Cannot restore '{label}'"))?;
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
    let rel = dest.strip_prefix(root)?;
    validate_restore_relpath(rel)?;
    let mut path = root.to_path_buf();
    for part in rel.components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_symlink() => {
                bail!(
                    "Refusing to restore through symbolic link '{}'",
                    path.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
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

    fn test_archive(path: &Path, entries: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, contents) in entries {
            if name.ends_with('/') {
                zip.add_directory(*name, options).unwrap();
            } else {
                zip.start_file(*name, options).unwrap();
                zip.write_all(contents.as_bytes()).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn backup_does_not_truncate_an_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().to_path_buf()).unwrap();
        let output = dir.path().join("note.md");
        fs::write(&output, "valuable note").unwrap();
        assert!(cmd_backup(&store, Some(output.clone())).is_err());
        assert_eq!(fs::read_to_string(output).unwrap(), "valuable note");
    }

    #[test]
    fn restore_corrupt_archive_preserves_all_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        store.write("first", "keep first").unwrap();
        store.write("second", "keep second").unwrap();
        let archive = dir.path().join("corrupt.zip");
        test_archive(
            &archive,
            &[("first.md", "replacement"), ("second.md", "corrupt-me")],
        );
        let mut bytes = fs::read(&archive).unwrap();
        let offset = bytes
            .windows(10)
            .position(|part| part == b"corrupt-me")
            .unwrap();
        bytes[offset] ^= 1;
        fs::write(&archive, bytes).unwrap();
        assert!(cmd_restore(&store, &archive, true).is_err());
        assert_eq!(store.read("first").unwrap(), "keep first");
        assert_eq!(store.read("second").unwrap(), "keep second");
    }

    #[test]
    fn restore_preserves_empty_folders() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        let archive = dir.path().join("folders.zip");
        test_archive(&archive, &[("empty/nested/", "")]);
        cmd_restore(&store, &archive, false).unwrap();
        assert!(store.root.join("empty/nested").is_dir());
    }

    #[test]
    fn restore_rejects_file_directory_overlap_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        let archive = dir.path().join("overlap.zip");
        test_archive(
            &archive,
            &[("parent", "file"), ("parent/child.md", "child")],
        );
        assert!(cmd_restore(&store, &archive, true).is_err());
        assert!(!store.root.join("parent").exists());
    }

    #[test]
    fn restore_rejects_case_aliases_on_case_insensitive_volumes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CaseProbe"), "").unwrap();
        if !dir.path().join("caseprobe").exists() {
            return; // This filesystem treats the two paths as distinct files.
        }
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        let archive = dir.path().join("aliases.zip");
        for entries in [
            [("Note.md", "first"), ("note.md", "second")],
            [("Parent", "file"), ("parent/child.md", "child")],
            [("parent/child.md", "child"), ("Parent", "file")],
        ] {
            test_archive(&archive, &entries);
            assert!(cmd_restore(&store, &archive, true).is_err());
            assert_eq!(fs::read_dir(&store.root).unwrap().count(), 0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn restore_preserves_permissions_and_refuses_readonly_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        store.write("note", "original").unwrap();
        let path = store.path_for("note");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let archive = dir.path().join("permissions.zip");
        test_archive(&archive, &[("note.md", "restored")]);
        cmd_restore(&store, &archive, true).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        test_archive(&archive, &[("first.md", "new"), ("note.md", "replacement")]);
        assert!(cmd_restore(&store, &archive, true).is_err());
        assert!(!store.exists("first"));
        assert_eq!(store.read("note").unwrap(), "restored");
    }

    #[cfg(unix)]
    #[test]
    fn restore_rejects_symlink_ancestors_and_leaf_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        let outside = dir.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("private.md"), "private").unwrap();
        std::os::unix::fs::symlink(&outside, store.root.join("link")).unwrap();
        std::os::unix::fs::symlink(outside.join("private.md"), store.root.join("leaf.md")).unwrap();
        for name in ["link/new/note.md", "leaf.md"] {
            let archive = dir.path().join("symlink.zip");
            test_archive(&archive, &[(name, "replacement")]);
            assert!(cmd_restore(&store, &archive, true).is_err());
        }
        assert!(!outside.join("new").exists());
        assert_eq!(
            fs::read_to_string(outside.join("private.md")).unwrap(),
            "private"
        );
    }

    #[cfg(unix)]
    #[test]
    fn backup_rejects_symlink_loops_without_publishing_partial_archive() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::with_root(dir.path().join("vault")).unwrap();
        std::os::unix::fs::symlink(&store.root, store.root.join("loop")).unwrap();
        let archive = dir.path().join("backup.zip");
        assert!(cmd_backup(&store, Some(archive.clone())).is_err());
        assert!(!archive.exists());
    }

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
