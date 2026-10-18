// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::actions::{reveal_folder, schedule_search, toggle_expand_all};
use crate::components::tree::VaultTree;
use crate::icons::{
    IconArrowUpDown, IconCheck, IconChevronsUpDown, IconFile, IconFolder, IconFolderPlus,
    IconSquarePen,
};
use crate::models::SearchHit;
use crate::ops::{AppOp, Ops};
use crate::state::{AppCtx, ModalState, TreeSort};
use crate::util::sort_vault_tree;

const LOGO: Asset = asset!("/assets/cobblestone.png");

#[component]
pub fn Sidebar() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let search = ctx.search_query;
    let tree = ctx.tree;
    let search_results = ctx.search_results;
    let active = ctx.active_slug;
    let tree_sort = ctx.tree_sort;
    let mut sort_open = use_signal(|| false);

    let sorted_tree = {
        let mut nodes = tree.read().clone();
        sort_vault_tree(&mut nodes, *tree_sort.read());
        nodes
    };

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-header",
                span { class: "logo",
                    img { class: "app-icon", src: LOGO, alt: "", width: "18", height: "18" }
                    span { "Cobblestone" }
                }
            }
            div { class: "search-wrap",
                input {
                    id: "search",
                    class: "search-input",
                    r#type: "text",
                    placeholder: "Search notes and folders…",
                    autocomplete: "off",
                    value: "{search}",
                    oninput: move |e| schedule_search(ctx, ops, e.value()),
                }
            }
            div { class: "sidebar-actions",
                div { class: "sidebar-toolbar",
                    button {
                        class: "icon-tool-btn",
                        r#type: "button",
                        title: "New note",
                        aria_label: "New note",
                        onclick: move |_| {
                            sort_open.set(false);
                            ctx.modal.set(ModalState::NewNote {
                                folder: None,
                                title: search.read().trim().to_string(),
                            });
                        },
                        IconSquarePen {}
                    }
                    button {
                        class: "icon-tool-btn",
                        r#type: "button",
                        title: "New folder",
                        aria_label: "New folder",
                        onclick: move |_| {
                            sort_open.set(false);
                            ctx.modal.set(ModalState::NewFolder {
                                parent: None,
                                name: String::new(),
                            });
                        },
                        IconFolderPlus {}
                    }
                    div { class: "sort-wrap",
                        button {
                            class: if *sort_open.read() {
                                "icon-tool-btn active"
                            } else {
                                "icon-tool-btn"
                            },
                            r#type: "button",
                            title: "Change sort order",
                            aria_label: "Change sort order",
                            aria_expanded: if *sort_open.read() { "true" } else { "false" },
                            onclick: move |e| {
                                e.stop_propagation();
                                let open = *sort_open.peek();
                                sort_open.set(!open);
                            },
                            IconArrowUpDown {}
                        }
                        if *sort_open.read() {
                            div {
                                class: "sort-menu",
                                role: "menu",
                                onclick: move |e| e.stop_propagation(),
                                SortMenuItems { tree_sort, sort_open }
                            }
                        }
                    }
                    button {
                        class: "icon-tool-btn",
                        r#type: "button",
                        title: "Expand or collapse all folders",
                        aria_label: "Expand or collapse all folders",
                        onclick: move |_| {
                            sort_open.set(false);
                            toggle_expand_all(ctx);
                        },
                        IconChevronsUpDown {}
                    }
                }
            }
            if *sort_open.read() {
                div {
                    class: "sort-menu-catcher",
                    onclick: move |_| sort_open.set(false),
                }
            }
            div { class: "note-list", id: "note-list",
                if let Some(results) = search_results.read().clone() {
                    if results.is_empty() {
                        div { class: "empty-state", "No matching notes or folders" }
                    } else {
                        for hit in results.iter() {
                            match hit {
                                SearchHit::Folder { path, name } => {
                                    let path = path.clone();
                                    let name = name.clone();
                                    rsx! {
                                        div {
                                            class: "note-item",
                                            onclick: move |_| reveal_folder(ctx, path.clone()),
                                            div { class: "note-item-title",
                                                span { class: "tree-folder-icon", style: "display:inline-grid;margin-right:6px;vertical-align:middle;",
                                                    IconFolder {}
                                                }
                                                "{name}"
                                            }
                                            div { class: "note-item-meta", "Folder · {path}" }
                                        }
                                    }
                                }
                                SearchHit::Note(note) => {
                                    let slug = note.slug.clone();
                                    let title = note.title.clone();
                                    let modified = note.modified.clone();
                                    let is_active = active.read().as_deref() == Some(slug.as_str());
                                    rsx! {
                                        div {
                                            class: if is_active { "note-item active" } else { "note-item" },
                                            onclick: move |_| {
                                                ops.send(AppOp::OpenNote(slug.clone()));
                                            },
                                            div { class: "note-item-title",
                                                span { class: "tree-note-icon", style: "display:inline-grid;margin-right:6px;vertical-align:middle;",
                                                    IconFile {}
                                                }
                                                "{title}"
                                            }
                                            div { class: "note-item-meta", "{modified}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if sorted_tree.is_empty() {
                    div { class: "empty-state", "No notes yet" }
                } else {
                    VaultTree { nodes: sorted_tree }
                }
            }
        }
    }
}

#[component]
fn SortMenuItems(
    mut tree_sort: Signal<TreeSort>,
    mut sort_open: Signal<bool>,
) -> Element {
    let current = *tree_sort.read();
    rsx! {
        for mode in TreeSort::ALL {
            if needs_sort_separator(mode) {
                div { class: "tree-context-separator" }
            }
            button {
                class: if current == mode {
                    "sort-menu-item selected"
                } else {
                    "sort-menu-item"
                },
                r#type: "button",
                role: "menuitemradio",
                aria_checked: if current == mode { "true" } else { "false" },
                onclick: move |_| {
                    tree_sort.set(mode);
                    sort_open.set(false);
                },
                span { class: "sort-menu-check",
                    if current == mode {
                        IconCheck {}
                    }
                }
                span { "{mode.label()}" }
            }
        }
    }
}

fn needs_sort_separator(mode: TreeSort) -> bool {
    matches!(
        mode,
        TreeSort::ModifiedNew | TreeSort::CreatedNew
    )
}
