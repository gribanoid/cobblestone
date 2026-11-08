// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use super::settings::SettingsModal;
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
        ModalState::Projects => rsx! { ProjectsModal {} },
        ModalState::Settings => rsx! { SettingsModal {} },
        ModalState::CommandPalette => rsx! { CommandPaletteModal {} },
        ModalState::PromptValue {
            title,
            default,
            pending_command,
        } => rsx! {
            PromptValueModal {
                title,
                default_value: default,
                pending_command,
            }
        },
    }
}

#[component]
fn PromptValueModal(title: String, default_value: String, pending_command: String) -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let mut value = use_signal(|| default_value.clone());
    let cmd = pending_command.clone();

    rsx! {
        div { class: "modal-backdrop", style: "display:flex",
            div { class: "modal",
                h3 { "{title}" }
                input {
                    class: "modal-input",
                    r#type: "text",
                    value: "{value}",
                    onmounted: move |e| { drop(e.data().set_focus(true)); },
                    oninput: move |e| value.set(e.value()),
                    onkeydown: move |e| {
                        if e.data().key() == Key::Enter {
                            ops.send(AppOp::ConfirmPluginPrompt {
                                command: cmd.clone(),
                                value: value.read().clone(),
                            });
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
                            ops.send(AppOp::ConfirmPluginPrompt {
                                command: pending_command.clone(),
                                value: value.read().clone(),
                            });
                        },
                        "OK"
                    }
                }
            }
        }
    }
}

#[component]
fn CommandPaletteModal() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let mut filter = use_signal(String::new);
    let commands = use_signal(Vec::<(String, String)>::new);

    #[cfg(feature = "desktop")]
    {
        use cobblestone_plugin_host::SharedPluginHost;
        let mut commands = commands;
        let host = try_use_context::<SharedPluginHost>();
        use_effect(move || {
            if let Some(host) = host.clone()
                && let Ok(h) = host.lock()
            {
                let mut list = h.list_commands();
                list.sort_by(|a, b| a.1.cmp(&b.1));
                commands.set(list);
            }
        });
    }

    let q = filter.read().trim().to_lowercase();
    let shown: Vec<(String, String)> = commands
        .read()
        .iter()
        .filter(|(id, name)| {
            q.is_empty() || id.to_lowercase().contains(&q) || name.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();

    rsx! {
        div { class: "modal-backdrop", id: "command-palette", style: "display:flex",
            div { class: "modal projects-modal",
                h3 { "Commands" }
                p { class: "modal-hint", "First-party plugin commands (⌘P). Also: cb plugin run <id>" }
                input {
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Filter…",
                    value: "{filter}",
                    onmounted: move |e| { drop(e.data().set_focus(true)); },
                    oninput: move |e| filter.set(e.value()),
                }
                div { class: "projects-list",
                    for (id, name) in shown {
                        {
                            let id_run = id.clone();
                            rsx! {
                                button {
                                    class: "project-row",
                                    r#type: "button",
                                    onclick: move |_| {
                                        ops.send(AppOp::RunPluginCommand(id_run.clone()));
                                    },
                                    div { class: "project-row-name", "{name}" }
                                    div { class: "project-row-path", "{id}" }
                                }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    button {
                        class: "tb-btn",
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Close"
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectsModal() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let backend = use_context::<crate::backend::SharedBackend>();
    let mut projects = use_signal(Vec::<(String, String)>::new);
    let mut new_name = use_signal(String::new);
    let mut new_path = use_signal(String::new);
    let mut current_path = use_signal(String::new);

    use_effect({
        let backend = backend.clone();
        move || {
            let backend = backend.clone();
            spawn(async move {
                if let Ok(list) = backend.list_projects().await {
                    projects.set(list);
                }
                if let Ok(p) = backend.vault_path().await {
                    current_path.set(p);
                }
            });
        }
    });

    rsx! {
        div { class: "modal-backdrop", id: "projects-modal", style: "display:flex",
            div { class: "modal projects-modal",
                h3 { "Projects" }
                p { class: "modal-hint", "Open a Markdown folder as a Cobblestone project (local vault)." }
                div { class: "projects-list",
                    for (name, path) in projects.read().clone() {
                        {
                            let path_open = path.clone();
                            let active = *current_path.read() == path;
                            rsx! {
                                button {
                                    class: if active { "project-row active" } else { "project-row" },
                                    r#type: "button",
                                    onclick: move |_| {
                                        ops.send(AppOp::SwitchVault(path_open.clone()));
                                    },
                                    div { class: "project-row-name", "{name}" }
                                    div { class: "project-row-path", "{path}" }
                                }
                            }
                        }
                    }
                }
                h4 { class: "projects-create-title", "Create / open folder" }
                p { class: "modal-hint", "Or click a project above to switch. Both fields are required to add a new one." }
                input {
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Project name",
                    value: "{new_name}",
                    oninput: move |e| new_name.set(e.value()),
                }
                input {
                    class: "modal-input",
                    r#type: "text",
                    placeholder: "Absolute folder path",
                    value: "{new_path}",
                    oninput: move |e| new_path.set(e.value()),
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
                            let name = new_name.peek().trim().to_string();
                            let path = new_path.peek().trim().to_string();
                            if name.is_empty() || path.is_empty() {
                                ctx.show_error("Enter project name and absolute folder path");
                                return;
                            }
                            let backend = backend.clone();
                            let ops = ops;
                            let mut ctx = ctx;
                            spawn(async move {
                                if let Err(e) = backend.add_project(&name, &path).await {
                                    ctx.show_error(e);
                                    return;
                                }
                                ops.send(AppOp::SwitchVault(path));
                            });
                        },
                        "Add & open"
                    }
                }
            }
        }
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
