// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::ops::{AppOp, Ops};
use crate::state::{AppCtx, ModalState, RenameKind};

#[component]
pub fn Modals() -> Element {
    let ctx = use_context::<AppCtx>();
    match ctx.modal.read().clone() {
        ModalState::None => rsx! {},
        ModalState::NewNote { folder, title } => rsx! { NewNoteModal { folder, title } },
        ModalState::NewFolder { parent, name } => rsx! { NewFolderModal { parent, name } },
        ModalState::Rename { kind, value } => rsx! { RenameModal { kind, value } },
        ModalState::Delete { title, message, .. } => rsx! { DeleteModal { title, message } },
    }
}

#[component]
fn NewNoteModal(folder: Option<String>, title: String) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let mut draft = use_signal(|| title);
    let hint = folder.clone().unwrap_or_else(|| "Root".into());

    rsx! {
        div { class: "modal-backdrop", id: "note-modal", style: "display:flex",
            div { class: "modal",
                h3 { "New note" }
                p { id: "note-modal-hint", "Folder: {hint}" }
                input {
                    id: "note-title-input",
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Note title",
                    autocomplete: "off",
                    value: "{draft}",
                    onmounted: move |e| { drop(e.data().set_focus(true)); },
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: {
                        let folder = folder.clone();
                        move |e| {
                            if e.data().key() == Key::Enter {
                                e.prevent_default();
                                submit_new_note(ops, draft.peek().clone(), folder.clone());
                            }
                        }
                    },
                }
                div { class: "modal-actions",
                    button {
                        class: "tb-btn",
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Cancel"
                    }
                    button {
                        class: "tb-btn primary",
                        onclick: move |_| {
                            submit_new_note(ops, draft.peek().clone(), folder.clone());
                        },
                        "Create"
                    }
                }
            }
        }
    }
}

fn submit_new_note(ops: Ops, raw: String, folder: Option<String>) {
    let title = {
        let t = raw.trim();
        if t.is_empty() {
            format!("Note {}", now_ms())
        } else {
            t.to_string()
        }
    };
    ops.send(AppOp::CreateNote { title, folder });
}

#[component]
fn NewFolderModal(parent: Option<String>, name: String) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let mut draft = use_signal(|| name);
    let hint = parent.clone().unwrap_or_else(|| "Root".into());

    rsx! {
        div { class: "modal-backdrop", id: "folder-modal", style: "display:flex",
            div { class: "modal",
                h3 { "New folder" }
                p { id: "folder-modal-hint", "Parent: {hint}" }
                input {
                    id: "folder-name-input",
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Folder name",
                    autocomplete: "off",
                    value: "{draft}",
                    onmounted: move |e| { drop(e.data().set_focus(true)); },
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: {
                        let parent = parent.clone();
                        move |e| {
                            if e.data().key() == Key::Enter {
                                e.prevent_default();
                                let n = draft.peek().trim().to_string();
                                if !n.is_empty() {
                                    ops.send(AppOp::CreateFolder {
                                        name: n,
                                        parent: parent.clone(),
                                    });
                                }
                            }
                        }
                    },
                }
                div { class: "modal-actions",
                    button {
                        class: "tb-btn",
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Cancel"
                    }
                    button {
                        class: "tb-btn primary",
                        onclick: move |_| {
                            let n = draft.peek().trim().to_string();
                            if !n.is_empty() {
                                ops.send(AppOp::CreateFolder {
                                    name: n,
                                    parent: parent.clone(),
                                });
                            }
                        },
                        "Create"
                    }
                }
            }
        }
    }
}

#[component]
fn RenameModal(kind: RenameKind, value: String) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let mut draft = use_signal(|| value);
    let title = match kind {
        RenameKind::Note { .. } => "Rename note",
        RenameKind::Folder { .. } => "Rename folder",
    };

    rsx! {
        div { class: "modal-backdrop", id: "rename-modal", style: "display:flex",
            div { class: "modal",
                h3 { id: "rename-modal-title", "{title}" }
                input {
                    id: "rename-input",
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Name",
                    autocomplete: "off",
                    value: "{draft}",
                    onmounted: move |e| { drop(e.data().set_focus(true)); },
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: move |e| {
                        if e.data().key() == Key::Enter {
                            e.prevent_default();
                            ops.send(AppOp::ConfirmRename(draft.peek().clone()));
                        }
                    },
                }
                div { class: "modal-actions",
                    button {
                        class: "tb-btn",
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Cancel"
                    }
                    button {
                        class: "tb-btn primary",
                        onclick: move |_| {
                            ops.send(AppOp::ConfirmRename(draft.peek().clone()));
                        },
                        "Rename"
                    }
                }
            }
        }
    }
}

#[component]
fn DeleteModal(title: String, message: String) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    rsx! {
        div { class: "modal-backdrop", id: "confirm-modal", style: "display:flex",
            div { class: "modal",
                h3 { id: "confirm-modal-title", "{title}" }
                p { id: "confirm-modal-message", "{message}" }
                div { class: "modal-actions",
                    button {
                        class: "tb-btn",
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Cancel"
                    }
                    button {
                        class: "tb-btn danger",
                        onclick: move |_| ops.send(AppOp::ConfirmDelete),
                        "Delete"
                    }
                }
            }
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
