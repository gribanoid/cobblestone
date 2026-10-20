// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::backend::SharedBackend;
use crate::models::NoteGraph;
use crate::models::SearchHit;
use crate::ops::{AppOp, Ops};
use crate::state::{
    AppCtx, ContextAction, ContextMenuItem, ContextMenuState, DeleteKind, DragPayload, ModalState,
    RenameKind,
};
use crate::timeutil::delay_ms;
use crate::util::{
    can_drop_note_or_folder, collect_folder_paths, content_with_title, expand_folder_path,
    find_matching_folders, flatten_tree, remap_path, strip_leading_heading,
};

pub async fn reload_tree(mut ctx: AppCtx, backend: SharedBackend) {
    match backend.list_tree().await {
        Ok(tree) => {
            let notes = flatten_tree(&tree);
            ctx.tree.set(tree);
            ctx.notes.set(notes);
            if ctx.search_query.peek().trim().is_empty() {
                ctx.search_results.set(None);
            }
        }
        Err(e) => ctx.show_error(e),
    }
}

pub async fn run_search(mut ctx: AppCtx, backend: SharedBackend) {
    let seq = *ctx.search_gen.peek();
    let q = ctx.search_query.peek().trim().to_string();
    if q.is_empty() {
        if seq == *ctx.search_gen.peek() {
            ctx.search_results.set(None);
            reload_tree(ctx, backend).await;
        }
        return;
    }
    match backend.search_notes(&q).await {
        Ok(notes) => {
            if seq != *ctx.search_gen.peek() {
                return;
            }
            let mut hits: Vec<SearchHit> = find_matching_folders(&ctx.tree.peek(), &q)
                .into_iter()
                .map(|(path, name)| SearchHit::Folder { path, name })
                .collect();
            hits.extend(notes.into_iter().map(SearchHit::Note));
            ctx.search_results.set(Some(hits));
        }
        Err(e) => {
            if seq == *ctx.search_gen.peek() {
                ctx.show_error(e);
            }
        }
    }
}

/// Leave search mode and expand the folder (and parents) in the vault tree.
pub fn reveal_folder(mut ctx: AppCtx, path: String) {
    {
        let mut expanded = ctx.expanded.write();
        expand_folder_path(&mut expanded, &path);
    }
    ctx.search_query.set(String::new());
    ctx.search_results.set(None);
    let mut g = ctx.search_gen.write();
    *g += 1;
}

pub async fn flush_save(mut ctx: AppCtx, backend: SharedBackend) -> bool {
    if !*ctx.is_dirty.peek() {
        return true;
    }
    let Some(slug_at_start) = ctx.active_slug.peek().clone() else {
        return true;
    };
    let title = ctx.active_title.peek().trim().to_string();
    if title.is_empty() {
        ctx.show_error("Note title cannot be empty");
        return false;
    }
    let content = ctx.active_content.peek().clone();
    let persisted = ctx.persisted_title.peek().clone();

    let mut slug = slug_at_start.clone();
    if persisted != title {
        match backend.rename_note(&slug_at_start, &title).await {
            Ok(new_slug) => {
                if ctx.active_slug.peek().as_deref() == Some(slug_at_start.as_str()) {
                    ctx.active_slug.set(Some(new_slug.clone()));
                    ctx.active_title.set(title.clone());
                }
                slug = new_slug;
            }
            Err(e) => {
                ctx.show_error(e);
                return false;
            }
        }
    }

    match backend
        .save_note(&slug, &content_with_title(&content, &title))
        .await
    {
        Ok(()) => {
            if ctx.active_slug.peek().as_deref() == Some(slug.as_str()) {
                ctx.active_content.set(content);
                ctx.persisted_title.set(title);
                ctx.is_dirty.set(false);
            }
            reload_tree(ctx, backend.clone()).await;
            refresh_graph(ctx, backend, &slug).await;
            true
        }
        Err(e) => {
            ctx.show_error(e);
            false
        }
    }
}

pub async fn refresh_graph(mut ctx: AppCtx, backend: SharedBackend, slug: &str) {
    match backend.note_graph(slug).await {
        Ok(g) => {
            if ctx.active_slug.peek().as_deref() == Some(slug) {
                ctx.graph.set(Some(g));
            }
        }
        Err(_) => {
            if ctx.active_slug.peek().as_deref() == Some(slug) {
                ctx.graph.set(Some(NoteGraph::default()));
            }
        }
    }
}

pub async fn open_note(mut ctx: AppCtx, backend: SharedBackend, slug: String) {
    if *ctx.busy.peek() {
        return;
    }
    ctx.busy.set(true);
    if !flush_save(ctx, backend.clone()).await {
        ctx.busy.set(false);
        return;
    }
    if ctx.active_slug.peek().as_deref() == Some(slug.as_str()) {
        ctx.busy.set(false);
        return;
    }
    match backend.get_note(&slug).await {
        Ok(note) => {
            ctx.active_slug.set(Some(slug.clone()));
            ctx.active_title.set(note.title.clone());
            ctx.persisted_title.set(note.title);
            ctx.active_content.set(strip_leading_heading(&note.content));
            ctx.is_dirty.set(false);
            ctx.graph.set(None);
            refresh_graph(ctx, backend, &slug).await;
        }
        Err(e) => ctx.show_error(e),
    }
    ctx.busy.set(false);
}

pub async fn create_note(
    mut ctx: AppCtx,
    backend: SharedBackend,
    title: String,
    folder: Option<String>,
) {
    if *ctx.busy.peek() {
        return;
    }
    ctx.busy.set(true);
    ctx.modal.set(ModalState::None);
    if !flush_save(ctx, backend.clone()).await {
        ctx.busy.set(false);
        return;
    }
    match backend.create_note(&title, folder.as_deref()).await {
        Ok(slug) => {
            ctx.search_query.set(String::new());
            ctx.search_results.set(None);
            if let Some(f) = &folder {
                ctx.expanded.write().insert(f.clone());
            }
            reload_tree(ctx, backend.clone()).await;
            open_note(ctx, backend, slug).await;
        }
        Err(e) => ctx.show_error(e),
    }
    ctx.busy.set(false);
}

pub async fn create_folder(
    mut ctx: AppCtx,
    backend: SharedBackend,
    name: String,
    parent: Option<String>,
) {
    if name.trim().is_empty() {
        return;
    }
    ctx.modal.set(ModalState::None);
    let path = match &parent {
        Some(p) => format!("{p}/{name}"),
        None => name,
    };
    match backend.create_folder(&path).await {
        Ok(()) => {
            if let Some(p) = &parent {
                ctx.expanded.write().insert(p.clone());
            }
            ctx.expanded.write().insert(path);
            reload_tree(ctx, backend).await;
        }
        Err(e) => ctx.show_error(e),
    }
}

pub async fn confirm_rename(mut ctx: AppCtx, backend: SharedBackend, value: String) {
    let ModalState::Rename { kind, .. } = ctx.modal.peek().clone() else {
        return;
    };
    let name = value.trim().to_string();
    if name.is_empty() {
        return;
    }
    ctx.modal.set(ModalState::None);
    if !flush_save(ctx, backend.clone()).await {
        return;
    }
    match kind {
        RenameKind::Note { slug } => match backend.rename_note(&slug, &name).await {
            Ok(new_slug) => {
                if ctx.active_slug.peek().as_deref() == Some(slug.as_str()) {
                    ctx.active_slug.set(Some(new_slug));
                    ctx.active_title.set(name.clone());
                    ctx.persisted_title.set(name);
                    ctx.is_dirty.set(false);
                }
                reload_tree(ctx, backend).await;
            }
            Err(e) => ctx.show_error(e),
        },
        RenameKind::Folder { path } => match backend.rename_folder(&path, &name).await {
            Ok(new_path) => {
                let expanded: HashSet<_> = ctx
                    .expanded
                    .peek()
                    .iter()
                    .map(|p| remap_path(p, &path, &new_path))
                    .collect();
                ctx.expanded.set(expanded);
                let active = ctx.active_slug.peek().clone();
                if let Some(slug) = active
                    && slug.starts_with(&format!("{path}/"))
                {
                    ctx.active_slug
                        .set(Some(remap_path(&slug, &path, &new_path)));
                }
                reload_tree(ctx, backend).await;
            }
            Err(e) => ctx.show_error(e),
        },
    }
}

pub async fn confirm_delete(mut ctx: AppCtx, backend: SharedBackend) {
    let ModalState::Delete { kind, .. } = ctx.modal.peek().clone() else {
        return;
    };
    ctx.modal.set(ModalState::None);
    if !flush_save(ctx, backend.clone()).await {
        return;
    }
    match kind {
        DeleteKind::Note { slug, was_active } => {
            let slug = if was_active {
                ctx.active_slug
                    .peek()
                    .clone()
                    .unwrap_or_else(|| slug.clone())
            } else {
                slug
            };
            match backend.delete_note(&slug).await {
                Ok(()) => {
                    if ctx.active_slug.peek().as_deref() == Some(slug.as_str()) {
                        clear_editor(ctx);
                    }
                    reload_tree(ctx, backend).await;
                }
                Err(e) => ctx.show_error(e),
            }
        }
        DeleteKind::Folder { path } => match backend.delete_folder(&path).await {
            Ok(()) => {
                let active = ctx.active_slug.peek().clone();
                if let Some(slug) = active
                    && (slug == path || slug.starts_with(&format!("{path}/")))
                {
                    clear_editor(ctx);
                }
                let expanded: HashSet<String> = ctx
                    .expanded
                    .peek()
                    .iter()
                    .filter(|p| *p != &path && !p.starts_with(&format!("{path}/")))
                    .cloned()
                    .collect();
                ctx.expanded.set(expanded);
                reload_tree(ctx, backend).await;
            }
            Err(e) => ctx.show_error(e),
        },
        DeleteKind::File { path } => match backend.delete_file(&path).await {
            Ok(()) => reload_tree(ctx, backend).await,
            Err(e) => ctx.show_error(e),
        },
    }
}

pub async fn move_note_to_folder(
    mut ctx: AppCtx,
    backend: SharedBackend,
    slug: String,
    folder: Option<String>,
) {
    if !can_drop_note_or_folder(true, Some(&slug), None, folder.as_deref()) {
        return;
    }
    if !flush_save(ctx, backend.clone()).await {
        return;
    }
    let moving_active = ctx.active_slug.peek().as_deref() == Some(slug.as_str());
    let move_slug = if moving_active {
        ctx.active_slug
            .peek()
            .clone()
            .unwrap_or_else(|| slug.clone())
    } else {
        slug
    };
    match backend.move_note(&move_slug, folder.as_deref()).await {
        Ok(new_slug) => {
            if let Some(f) = &folder {
                ctx.expanded.write().insert(f.clone());
            }
            if ctx.active_slug.peek().as_deref() == Some(move_slug.as_str()) {
                ctx.active_slug.set(Some(new_slug));
            }
            reload_tree(ctx, backend).await;
        }
        Err(e) => ctx.show_error(e),
    }
}

pub async fn move_folder_to_parent(
    mut ctx: AppCtx,
    backend: SharedBackend,
    folder_path: String,
    dest: Option<String>,
) {
    if !can_drop_note_or_folder(false, None, Some(&folder_path), dest.as_deref()) {
        return;
    }
    let affects_active = ctx
        .active_slug
        .peek()
        .as_ref()
        .is_some_and(|s| s == &folder_path || s.starts_with(&format!("{folder_path}/")));
    if affects_active && !flush_save(ctx, backend.clone()).await {
        return;
    }
    match backend.move_folder(&folder_path, dest.as_deref()).await {
        Ok(new_path) => {
            if let Some(d) = &dest {
                ctx.expanded.write().insert(d.clone());
            }
            ctx.expanded.write().insert(new_path.clone());
            let expanded: HashSet<_> = ctx
                .expanded
                .peek()
                .iter()
                .map(|p| remap_path(p, &folder_path, &new_path))
                .collect();
            ctx.expanded.set(expanded);
            let active = ctx.active_slug.peek().clone();
            if let Some(slug) = active {
                ctx.active_slug
                    .set(Some(remap_path(&slug, &folder_path, &new_path)));
            }
            reload_tree(ctx, backend).await;
        }
        Err(e) => ctx.show_error(e),
    }
}

pub async fn apply_drop(mut ctx: AppCtx, backend: SharedBackend, dest: Option<String>) {
    let Some(drag) = ctx.drag.peek().clone() else {
        return;
    };
    ctx.drag.set(None);
    match drag.payload {
        DragPayload::Note { slug } => move_note_to_folder(ctx, backend, slug, dest).await,
        DragPayload::Folder { path } => move_folder_to_parent(ctx, backend, path, dest).await,
    }
}

pub fn clear_editor(mut ctx: AppCtx) {
    ctx.active_slug.set(None);
    ctx.active_title.set(String::new());
    ctx.active_content.set(String::new());
    ctx.persisted_title.set(String::new());
    ctx.graph.set(None);
    ctx.is_dirty.set(false);
}

/// Where an attached file goes.
pub enum AttachTarget {
    /// Pasted / dropped into the open note: saved next to it, `![[name]]`
    /// inserted at the caret.
    ActiveNote,
    /// Dropped onto a sidebar folder (`""` = vault root): saved there only.
    Folder(String),
}

/// Save an image (Obsidian-style). Empty `filename` → `Pasted image <stamp>.png`.
/// For [`AttachTarget::ActiveNote`] the editor edit triggers autosave.
pub async fn attach_image(
    mut ctx: AppCtx,
    backend: SharedBackend,
    target: AttachTarget,
    filename: String,
    bytes: Vec<u8>,
) {
    let (folder, insert) = match target {
        AttachTarget::ActiveNote => {
            let Some(slug) = ctx.active_slug.peek().clone() else {
                ctx.show_error("Open a note to attach an image");
                return;
            };
            (slug.rsplit_once('/').map(|(dir, _)| dir.to_string()), true)
        }
        AttachTarget::Folder(f) => ((!f.is_empty()).then_some(f), false),
    };
    match backend
        .save_attachment(folder.as_deref(), &filename, &bytes)
        .await
    {
        Ok(rel) => {
            let name = rel.rsplit('/').next().unwrap_or(&rel);
            if insert {
                ctx.editor_insert.set(Some(format!("![[{name}]]")));
            }
            if let Some(f) = folder {
                expand_folder_path(&mut ctx.expanded.write(), &f);
            }
            reload_tree(ctx, backend).await;
        }
        Err(e) => ctx.show_error(format!("Could not save image: {e}")),
    }
}

/// Mark dirty and schedule a generation-guarded autosave (~0.8s).
pub fn mark_dirty(mut ctx: AppCtx, ops: Ops) {
    ctx.is_dirty.set(true);
    let seq = {
        let mut g = ctx.dirty_gen.write();
        *g += 1;
        *g
    };
    spawn(async move {
        delay_ms(800).await;
        if *ctx.dirty_gen.peek() == seq && *ctx.is_dirty.peek() {
            ops.send(AppOp::Save);
        }
    });
}

pub fn schedule_search(mut ctx: AppCtx, ops: Ops, query: String) {
    ctx.search_query.set(query);
    let seq = {
        let mut g = ctx.search_gen.write();
        *g += 1;
        *g
    };
    spawn(async move {
        delay_ms(200).await;
        if *ctx.search_gen.peek() == seq {
            ops.send(AppOp::Search);
        }
    });
}

pub fn toggle_folder(mut ctx: AppCtx, path: String) {
    let mut expanded = ctx.expanded.write();
    if !expanded.remove(&path) {
        expanded.insert(path);
    }
}

/// Expand every folder, or collapse all if any folder is already open.
pub fn toggle_expand_all(mut ctx: AppCtx) {
    let folders = collect_folder_paths(&ctx.tree.peek());
    if folders.is_empty() {
        return;
    }
    let any_open = {
        let expanded = ctx.expanded.peek();
        folders.iter().any(|p| expanded.contains(p))
    };
    if any_open {
        ctx.expanded.set(HashSet::new());
    } else {
        ctx.expanded.set(folders.into_iter().collect());
    }
}

pub fn open_context_menu(mut ctx: AppCtx, x: f64, y: f64, items: Vec<ContextMenuItem>) {
    ctx.context_menu.set(Some(ContextMenuState { x, y, items }));
}

pub fn close_context_menu(mut ctx: AppCtx) {
    ctx.context_menu.set(None);
}

pub fn handle_context_action(mut ctx: AppCtx, action: ContextAction) {
    close_context_menu(ctx);
    match action {
        ContextAction::NewNote { folder } => {
            if let Some(f) = &folder {
                ctx.expanded.write().insert(f.clone());
            }
            ctx.modal.set(ModalState::NewNote {
                folder,
                title: ctx.search_query.peek().trim().to_string(),
            });
        }
        ContextAction::NewFolder { parent } => {
            if let Some(p) = &parent {
                ctx.expanded.write().insert(p.clone());
            }
            ctx.modal.set(ModalState::NewFolder {
                parent,
                name: String::new(),
            });
        }
        ContextAction::RenameNote { slug } => {
            let title = ctx
                .notes
                .peek()
                .iter()
                .find(|n| n.slug == slug)
                .map(|n| n.title.clone())
                .unwrap_or_else(|| slug.rsplit('/').next().unwrap_or(&slug).to_string());
            ctx.modal.set(ModalState::Rename {
                kind: RenameKind::Note { slug },
                value: title,
            });
        }
        ContextAction::RenameFolder { path } => {
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            ctx.modal.set(ModalState::Rename {
                kind: RenameKind::Folder { path },
                value: name,
            });
        }
        ContextAction::DeleteNote { slug } => {
            let was_active = ctx.active_slug.peek().as_deref() == Some(slug.as_str());
            ctx.modal.set(ModalState::Delete {
                kind: DeleteKind::Note { slug, was_active },
                title: "Delete note?".into(),
                message:
                    "This will permanently remove the note from disk. This action cannot be undone."
                        .into(),
            });
        }
        ContextAction::DeleteFolder { path } => {
            ctx.modal.set(ModalState::Delete {
                kind: DeleteKind::Folder { path },
                title: "Delete folder?".into(),
                message: "This will permanently delete the folder and everything inside it.".into(),
            });
        }
        ContextAction::DeleteFile { path } => {
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            ctx.modal.set(ModalState::Delete {
                kind: DeleteKind::File { path },
                title: "Delete file?".into(),
                message: format!(
                    "'{name}' will be permanently removed from disk. Notes that embed it will show a missing image."
                ),
            });
        }
        ContextAction::InsertInNote { text } => {
            if ctx.active_slug.peek().is_some() {
                ctx.editor_insert.set(Some(text));
            } else {
                ctx.show_error("Open a note first");
            }
        }
        ContextAction::Copy { text } => {
            spawn(async move {
                if let Err(e) = copy_to_clipboard(&text).await {
                    ctx.show_error(e);
                }
            });
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or_else(|| "No window".to_string())?;
    let promise = window.navigator().clipboard().write_text(text);
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(feature = "desktop")]
    {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard
            .set_text(text.to_string())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = text;
        Err("Clipboard unavailable".into())
    }
}

pub fn note_menu_items(slug: &str) -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem {
            label: "Rename".into(),
            danger: false,
            action: ContextAction::RenameNote {
                slug: slug.to_string(),
            },
        },
        ContextMenuItem {
            label: "Copy path".into(),
            danger: false,
            action: ContextAction::Copy {
                text: slug.to_string(),
            },
        },
        ContextMenuItem {
            label: "Delete".into(),
            danger: true,
            action: ContextAction::DeleteNote {
                slug: slug.to_string(),
            },
        },
    ]
}

pub fn file_menu_items(path: &str) -> Vec<ContextMenuItem> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let embed = format!("![[{name}]]");
    vec![
        ContextMenuItem {
            label: "Insert into note".into(),
            danger: false,
            action: ContextAction::InsertInNote {
                text: embed.clone(),
            },
        },
        ContextMenuItem {
            label: "Copy embed link".into(),
            danger: false,
            action: ContextAction::Copy { text: embed },
        },
        ContextMenuItem {
            label: "Copy path".into(),
            danger: false,
            action: ContextAction::Copy {
                text: path.to_string(),
            },
        },
        ContextMenuItem {
            label: "Delete".into(),
            danger: true,
            action: ContextAction::DeleteFile {
                path: path.to_string(),
            },
        },
    ]
}

pub fn folder_menu_items(path: &str) -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem {
            label: "New note".into(),
            danger: false,
            action: ContextAction::NewNote {
                folder: Some(path.to_string()),
            },
        },
        ContextMenuItem {
            label: "New folder".into(),
            danger: false,
            action: ContextAction::NewFolder {
                parent: Some(path.to_string()),
            },
        },
        ContextMenuItem {
            label: "Rename".into(),
            danger: false,
            action: ContextAction::RenameFolder {
                path: path.to_string(),
            },
        },
        ContextMenuItem {
            label: "Copy path".into(),
            danger: false,
            action: ContextAction::Copy {
                text: path.to_string(),
            },
        },
        ContextMenuItem {
            label: "Delete".into(),
            danger: true,
            action: ContextAction::DeleteFolder {
                path: path.to_string(),
            },
        },
    ]
}

pub fn root_menu_items() -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem {
            label: "New note".into(),
            danger: false,
            action: ContextAction::NewNote { folder: None },
        },
        ContextMenuItem {
            label: "New folder".into(),
            danger: false,
            action: ContextAction::NewFolder { parent: None },
        },
    ]
}

pub fn toggle_theme(mut ctx: AppCtx) {
    let next = ctx.theme.peek().toggle();
    next.persist();
    ctx.theme.set(next);
}
