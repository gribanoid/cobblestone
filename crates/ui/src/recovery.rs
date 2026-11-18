// SPDX-License-Identifier: MIT

//! Synchronous draft journal: closing the native window must not discard the
//! text waiting for the autosave debounce or a failed save.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::backend::SharedBackend;
use crate::state::AppCtx;
use crate::util::content_with_title;

#[derive(Debug, Serialize, Deserialize)]
struct Draft {
    slug: String,
    title: String,
    persisted_title: String,
    content: String,
    saved_content: String,
}

fn draft_dir(root: &Path) -> Result<PathBuf, String> {
    let mut dir = root.to_path_buf();
    for part in [".cobblestone", "recovery"] {
        dir.push(part);
        match fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        if let Ok(meta) = fs::symlink_metadata(&dir)
            && (meta.file_type().is_symlink() || !meta.is_dir())
        {
            return Err("Draft directory must be a regular directory".into());
        }
    }
    Ok(dir)
}

fn regular_file(path: &Path) -> Result<(), String> {
    if let Ok(meta) = fs::symlink_metadata(path)
        && (meta.file_type().is_symlink() || !meta.is_file())
    {
        return Err("Draft journal must be a regular file".into());
    }
    Ok(())
}

fn validate_journal(path: &Path) -> Result<(), String> {
    let root = path.ancestors().nth(3).ok_or("Invalid draft path")?;
    draft_dir(root)?;
    regular_file(path)
}

/// A distinct journal and held OS lock for each window/project session. Other
/// processes can recover it only after this owner exits or switches projects.
pub(crate) struct Session {
    path: PathBuf,
    lock_path: PathBuf,
    _lock: fs::File,
}

impl Session {
    fn new(root: &Path) -> Result<Self, String> {
        let file = tempfile::Builder::new()
            .prefix("session-")
            .suffix(".lock")
            .tempfile_in(draft_dir(root)?)
            .map_err(|e| e.to_string())?;
        file.as_file().try_lock().map_err(|e| e.to_string())?;
        let (lock, lock_path) = file.keep().map_err(|e| e.to_string())?;
        Ok(Self {
            path: lock_path.with_extension("json"),
            lock_path,
            _lock: lock,
        })
    }

    fn clear(&self) -> Result<(), String> {
        validate_journal(&self.path)?;
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.path.exists() {
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}

fn abandoned(root: &Path) -> Result<Vec<Session>, String> {
    let mut sessions = Vec::new();
    for entry in fs::read_dir(draft_dir(root)?).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json")
            || !path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("session-")
        {
            continue;
        }
        regular_file(&path)?;
        let lock_path = path.with_extension("lock");
        regular_file(&lock_path)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| e.to_string())?;
        if lock.try_lock().is_ok() {
            sessions.push(Session {
                path,
                lock_path,
                _lock: lock,
            });
        }
    }
    sessions.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(sessions)
}

pub(crate) fn begin(mut ctx: AppCtx) {
    ctx.recovery_session.set(None);
    let Some(root) = ctx.project_path.peek().clone() else {
        return;
    };
    match Session::new(Path::new(&root)) {
        Ok(session) => ctx.recovery_session.set(Some(Arc::new(session))),
        Err(e) => ctx.show_error(format!("Could not start draft protection: {e}")),
    }
}

fn write_draft(path: &Path, draft: &Draft) -> Result<(), String> {
    // Recheck the parents on every write; a project may be changed externally.
    validate_journal(path)?;
    let bytes = serde_json::to_vec(draft).map_err(|e| e.to_string())?;
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_draft(path: &Path) -> Result<Option<Draft>, String> {
    validate_journal(path)?;
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn persist(mut ctx: AppCtx) -> bool {
    let (Some(session), Some(slug)) = (
        ctx.recovery_session.peek().clone(),
        ctx.active_slug.peek().clone(),
    ) else {
        return false;
    };
    let draft = Draft {
        slug,
        title: ctx.active_title.peek().clone(),
        persisted_title: ctx.persisted_title.peek().clone(),
        content: ctx.active_content.peek().clone(),
        saved_content: ctx.saved_content.peek().clone(),
    };
    if let Err(e) = write_draft(&session.path, &draft) {
        ctx.show_error(format!("Could not protect unsaved draft: {e}"));
        return false;
    }
    true
}

pub(crate) fn clear(mut ctx: AppCtx) {
    let Some(session) = ctx.recovery_session.peek().clone() else {
        return;
    };
    if let Err(e) = session.clear() {
        ctx.show_error(format!(
            "Note saved, but the draft journal could not be cleared: {e}"
        ));
    }
}

pub(crate) async fn restore(mut ctx: AppCtx, backend: SharedBackend) {
    if ctx.active_slug.peek().is_some() {
        return;
    }
    let Some(root) = ctx.project_path.peek().clone() else {
        return;
    };
    let sessions = match abandoned(Path::new(&root)) {
        Ok(sessions) => sessions,
        Err(e) => {
            ctx.show_error(format!("Could not find recovery drafts: {e}"));
            return;
        }
    };
    for session in sessions {
        if restore_session(ctx, backend.clone(), session).await {
            break;
        }
    }
}

async fn restore_session(mut ctx: AppCtx, backend: SharedBackend, session: Session) -> bool {
    let mut draft = match read_draft(&session.path) {
        Ok(Some(draft)) => draft,
        Ok(None) => return false,
        Err(e) => {
            ctx.show_error(format!("Could not recover unsaved draft: {e}"));
            return false;
        }
    };
    let disk = backend.get_note(&draft.slug).await.ok();
    let rendered = content_with_title(&draft.content, draft.title.trim());
    if disk.as_ref().is_some_and(|note| note.content == rendered) {
        let _ = session.clear();
        return false;
    }
    let conflict = disk
        .as_ref()
        .is_none_or(|note| note.content != draft.saved_content);
    if conflict {
        // Keep externally changed/deleted files intact. Create a separate note
        // with no-clobber semantics before making the recovered text editable.
        match recovery_copy(backend.clone(), &draft).await {
            Ok((slug, title, content)) => {
                draft.slug = slug;
                draft.title = title.clone();
                draft.persisted_title = title;
                draft.saved_content = content;
            }
            Err(e) => {
                ctx.show_error(format!(
                    "Draft is safe in .cobblestone/recovery; recovery failed: {e}"
                ));
                return false;
            }
        }
    }
    *ctx.editor_revision.write() += 1;
    *ctx.dirty_gen.write() += 1;
    ctx.active_slug.set(Some(draft.slug));
    ctx.active_title.set(draft.title);
    ctx.persisted_title.set(draft.persisted_title);
    ctx.active_content.set(draft.content);
    ctx.saved_content.set(draft.saved_content);
    ctx.is_dirty.set(true);
    ctx.editor_mode
        .set(ctx.editor_preferences.peek().editing_mode());
    if persist(ctx)
        && let Err(e) = session.clear()
    {
        ctx.show_error(format!("Recovered draft could not be retired: {e}"));
    }
    ctx.show_notice(if conflict {
        "Recovered unsaved changes into a separate note. The original file was left untouched."
    } else {
        "Recovered your unsaved changes. Save the note when you are ready."
    });
    crate::actions::reload_tree(ctx, backend).await;
    true
}

async fn recovery_copy(
    backend: SharedBackend,
    draft: &Draft,
) -> Result<(String, String, String), String> {
    let base: String = draft
        .title
        .chars()
        .take(60)
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let base = if base.trim().is_empty() {
        "Note"
    } else {
        base.trim()
    };
    for index in 1..=1000 {
        let title = if index == 1 {
            format!("{base} (recovered)")
        } else {
            format!("{base} (recovered {index})")
        };
        match backend.create_note(&title, None).await {
            Ok(slug) => {
                let content = format!("# {title}\n\n");
                return Ok((slug, title, content));
            }
            Err(_) if backend.get_note(&title).await.is_ok() => continue,
            Err(e) => return Err(e),
        }
    }
    Err("Too many recovered notes with the same title".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_replaces_previous_complete_draft() {
        let root = tempfile::tempdir().unwrap();
        let session = Session::new(root.path()).unwrap();
        let mut draft = Draft {
            slug: "Note".into(),
            title: "Note".into(),
            persisted_title: "Note".into(),
            content: "First edit".into(),
            saved_content: "# Note\n\nOriginal".into(),
        };
        write_draft(&session.path, &draft).unwrap();
        draft.content = "Latest edit".into();
        write_draft(&session.path, &draft).unwrap();
        assert_eq!(
            read_draft(&session.path).unwrap().unwrap().content,
            "Latest edit"
        );
        assert_eq!(
            fs::read_dir(root.path().join(".cobblestone/recovery"))
                .unwrap()
                .count(),
            2
        );
    }

    #[cfg(unix)]
    #[test]
    fn journal_rejects_symlink_directories_and_files() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let dir = root.path().join(".cobblestone");
        symlink(outside.path(), &dir).unwrap();
        assert!(Session::new(root.path()).is_err());
        assert!(abandoned(root.path()).is_err());
        fs::remove_file(&dir).unwrap();
        let session = Session::new(root.path()).unwrap();
        symlink(outside.path().join("private.json"), &session.path).unwrap();
        assert!(read_draft(&session.path).is_err());
        assert!(session.clear().is_err());
    }

    #[test]
    fn concurrent_sessions_cannot_clear_or_recover_each_others_drafts() {
        let root = tempfile::tempdir().unwrap();
        let first = Session::new(root.path()).unwrap();
        let second = Session::new(root.path()).unwrap();
        let mut draft = Draft {
            slug: "One".into(),
            title: "One".into(),
            persisted_title: "One".into(),
            content: "First draft".into(),
            saved_content: String::new(),
        };
        write_draft(&first.path, &draft).unwrap();
        draft.content = "Second draft".into();
        write_draft(&second.path, &draft).unwrap();
        assert!(abandoned(root.path()).unwrap().is_empty());
        first.clear().unwrap();
        assert_eq!(
            read_draft(&second.path).unwrap().unwrap().content,
            "Second draft"
        );
        let second_path = second.path.clone();
        drop(second);
        let recovered = abandoned(root.path()).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].path, second_path);
        assert!(abandoned(root.path()).unwrap().is_empty());
        recovered[0].clear().unwrap();
        assert!(!second_path.exists());
    }
}
