// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::actions::{
    folder_menu_items, note_menu_items, open_context_menu, root_menu_items, toggle_folder,
};
use super::drag_preview::suppress_native_drag_ghost;
use crate::icons::{IconChevron, IconFile, IconFolder};
use crate::models::VaultNode;
use crate::ops::{AppOp, Ops};
use crate::state::{AppCtx, DragPayload, DragState};
use crate::util::can_drop_note_or_folder;

fn track_drag_cursor(mut ctx: AppCtx, e: &Event<dioxus::html::DragData>) {
    let coords = e.data().client_coordinates();
    if let Some(mut d) = (ctx.drag)() {
        d.cursor_x = coords.x;
        d.cursor_y = coords.y;
        ctx.drag.set(Some(d));
    }
}

#[component]
pub fn VaultTree(nodes: Vec<VaultNode>) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let expanded = ctx.expanded;
    let active = ctx.active_slug;
    let drag = ctx.drag;

    rsx! {
        div {
            class: if drag.read().as_ref().is_some_and(|d| d.over_folder == Some(None)) {
                "vault-tree drag-over"
            } else {
                "vault-tree"
            },
            ondragover: move |e| {
                e.prevent_default();
                let coords = e.data().client_coordinates();
                if let Some(mut d) = drag() {
                    d.over_folder = Some(None);
                    d.cursor_x = coords.x;
                    d.cursor_y = coords.y;
                    ctx.drag.set(Some(d));
                }
            },
            ondrop: move |e| {
                e.prevent_default();
                ops.send(AppOp::Drop { dest: None });
            },
            oncontextmenu: move |e| {
                e.prevent_default();
                if !ctx.search_query.read().trim().is_empty() {
                    return;
                }
                open_context_menu(
                    ctx,
                    e.data().client_coordinates().x,
                    e.data().client_coordinates().y,
                    root_menu_items(),
                );
            },
            for node in nodes.iter() {
                TreeNode { node: node.clone(), expanded, active, drag }
            }
        }
    }
}

#[component]
fn TreeNode(
    node: VaultNode,
    expanded: Signal<std::collections::HashSet<String>>,
    active: Signal<Option<String>>,
    drag: Signal<Option<DragState>>,
) -> Element {
    match node {
        VaultNode::Folder {
            name,
            path,
            children,
        } => {
            rsx! {
                FolderNode {
                    name,
                    path,
                    child_nodes: children,
                    expanded,
                    active,
                    drag,
                }
            }
        }
        VaultNode::Note { slug, title, .. } => {
            rsx! {
                NoteNode { slug, title, active, drag }
            }
        }
    }
}

#[component]
fn FolderNode(
    name: String,
    path: String,
    child_nodes: Vec<VaultNode>,
    expanded: Signal<std::collections::HashSet<String>>,
    active: Signal<Option<String>>,
    drag: Signal<Option<DragState>>,
) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let is_open = expanded.read().contains(&path);
    let is_dragging = drag
        .read()
        .as_ref()
        .is_some_and(|d| d.is_folder_dragging(&path));
    let is_drag_over = drag.read().as_ref().is_some_and(|d| {
        d.over_folder == Some(Some(path.clone()))
            && match &d.payload {
                DragPayload::Note { slug } => {
                    can_drop_note_or_folder(true, Some(slug), None, Some(&path))
                }
                DragPayload::Folder { path: from } => {
                    can_drop_note_or_folder(false, None, Some(from), Some(&path))
                }
            }
    });

    let class = {
        let mut c = String::from("tree-folder");
        if is_open {
            c.push_str(" expanded");
        }
        if is_drag_over {
            c.push_str(" drag-over");
        }
        if is_dragging {
            c.push_str(" dragging");
        }
        c
    };

    rsx! {
        div {
            class: "tree-folder-wrap",
            "data-drop-folder": "{path}",
            div {
                class: "{class}",
                draggable: "true",
                ondragstart: {
                    let path = path.clone();
                    let name = name.clone();
                    move |e| {
                        suppress_native_drag_ghost(&e.data());
                        let coords = e.data().client_coordinates();
                        ctx.drag.set(Some(DragState {
                            payload: DragPayload::Folder { path: path.clone() },
                            label: name.clone(),
                            over_folder: None,
                            cursor_x: coords.x,
                            cursor_y: coords.y,
                        }));
                    }
                },
                ondrag: move |e| track_drag_cursor(ctx, &e),
                ondragend: move |_| ctx.drag.set(None),
                ondragover: {
                    let path = path.clone();
                    move |e| {
                        e.prevent_default();
                        e.stop_propagation();
                        let coords = e.data().client_coordinates();
                        if let Some(mut d) = drag() {
                            d.over_folder = Some(Some(path.clone()));
                            d.cursor_x = coords.x;
                            d.cursor_y = coords.y;
                            ctx.drag.set(Some(d));
                        }
                    }
                },
                ondrop: {
                    let path = path.clone();
                    move |e| {
                        e.prevent_default();
                        e.stop_propagation();
                        ops.send(AppOp::Drop {
                            dest: Some(path.clone()),
                        });
                    }
                },
                onclick: {
                    let path = path.clone();
                    move |_| toggle_folder(ctx, path.clone())
                },
                oncontextmenu: {
                    let path = path.clone();
                    move |e| {
                        e.prevent_default();
                        e.stop_propagation();
                        open_context_menu(
                            ctx,
                            e.data().client_coordinates().x,
                            e.data().client_coordinates().y,
                            folder_menu_items(&path),
                        );
                    }
                },
                span { class: "tree-chevron", IconChevron {} }
                span { class: "tree-folder-icon", IconFolder {} }
                span { class: "tree-folder-name", "{name}" }
            }
            if is_open {
                div { class: "tree-children",
                    for child in child_nodes.iter() {
                        TreeNode {
                            node: child.clone(),
                            expanded,
                            active,
                            drag,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NoteNode(
    slug: String,
    title: String,
    active: Signal<Option<String>>,
    drag: Signal<Option<DragState>>,
) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let is_active = active.read().as_deref() == Some(slug.as_str());
    let is_dragging = drag
        .read()
        .as_ref()
        .is_some_and(|d| d.is_note_dragging(&slug));

    let class = {
        let mut c = String::from("tree-note");
        if is_active {
            c.push_str(" active");
        }
        if is_dragging {
            c.push_str(" dragging");
        }
        c
    };

    rsx! {
        div {
            class: "{class}",
            draggable: "true",
            ondragstart: {
                let slug = slug.clone();
                let title = title.clone();
                move |e| {
                    suppress_native_drag_ghost(&e.data());
                    let coords = e.data().client_coordinates();
                    ctx.drag.set(Some(DragState {
                        payload: DragPayload::Note { slug: slug.clone() },
                        label: title.clone(),
                        over_folder: None,
                        cursor_x: coords.x,
                        cursor_y: coords.y,
                    }));
                }
            },
            ondrag: move |e| track_drag_cursor(ctx, &e),
            ondragend: move |_| ctx.drag.set(None),
            onclick: {
                let slug = slug.clone();
                move |_| ops.send(AppOp::OpenNote(slug.clone()))
            },
            oncontextmenu: {
                let slug = slug.clone();
                move |e| {
                    e.prevent_default();
                    e.stop_propagation();
                    open_context_menu(
                        ctx,
                        e.data().client_coordinates().x,
                        e.data().client_coordinates().y,
                        note_menu_items(&slug),
                    );
                }
            },
            span { class: "tree-spacer" }
            span { class: "tree-note-icon", IconFile {} }
            span { class: "tree-note-title", "{title}" }
        }
    }
}
