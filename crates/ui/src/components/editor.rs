// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use dioxus::dioxus_core::Task;
use dioxus::prelude::*;

use crate::actions::{mark_dirty, toggle_theme};
use crate::backend::SharedBackend;
use crate::components::brand::BrandIcon;
use crate::components::codemirror::{CodeMirrorEditor, use_codemirror};
use crate::components::md_editor::MarkdownEditor;
use crate::icons::{IconFile, IconPanelRight, IconSquarePen, IconSunMoon};
use crate::markdown::{embed_image_names, render_note_body};
use crate::models::NoteInfo;
use crate::ops::{AppOp, Ops};
use crate::state::{AppCtx, EditorMode, ModalState};

#[component]
pub fn EditorArea() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let backend = use_context::<SharedBackend>();
    let active = ctx.active_slug;
    let mut title = ctx.active_title;
    let mut content = ctx.active_content;
    let is_dirty = ctx.is_dirty;
    let mut panel_open = ctx.panel_open;
    let editor_mode = ctx.editor_mode;
    let mut note_menu_open = ctx.note_menu_open;
    let project_label = ctx.project_label;

    let codemirror_ready = use_codemirror();
    let mut image_srcs = use_signal(HashMap::<String, String>::new);
    let mut image_revision = use_signal(|| None);
    let mut pending_images = use_signal(|| None::<Task>);
    let image_names = use_memo(move || embed_image_names(&content.read()));

    use_effect(move || {
        let slug = active.read().clone();
        let revision = *ctx.editor_revision.read();
        let names = image_names.read().clone();
        let pending = *pending_images.peek();
        if let Some(task) = pending {
            task.cancel();
        }
        if *image_revision.peek() != Some(revision) {
            image_srcs.write().clear();
            image_revision.set(Some(revision));
        }
        if slug.is_none() || names.is_empty() {
            return;
        }
        let backend = backend.clone();
        pending_images.set(Some(spawn(async move {
            for name in names {
                if image_srcs.peek().contains_key(&name) {
                    continue;
                }
                let source = backend.attachment_src(&name).await;
                if *ctx.editor_revision.peek() != revision || *active.peek() != slug {
                    return;
                }
                if let Ok(src) = source {
                    image_srcs.write().insert(name, src);
                }
            }
        })));
    });

    if active.read().is_none() {
        let notes = ctx.notes.read();
        let history = ctx.recent_notes.read();
        let recent: Vec<_> = history
            .slugs()
            .filter_map(|slug| notes.iter().find(|note| note.slug == slug))
            .collect();
        let project = project_label.read();
        return rsx! {
            section { class: "welcome", id: "welcome",
                div { class: "welcome-content",
                    header { class: "welcome-heading",
                        BrandIcon {}
                        div {
                            h1 { "Cobblestone" }
                            if project.is_empty() || project.as_str() == "Cobblestone" {
                                p { class: "welcome-project", "Your Markdown workspace" }
                            } else {
                                p { class: "welcome-project", "{project}" }
                            }
                        }
                    }
                    div { class: "welcome-actions",
                        button {
                            class: "welcome-button welcome-primary",
                            r#type: "button",
                            onclick: move |_| ctx.modal.set(ModalState::NewNote {
                                folder: None,
                                title: String::new(),
                            }),
                            IconSquarePen {}
                            "New note"
                        }
                        if !notes.is_empty() {
                            button {
                                class: "welcome-button",
                                r#type: "button",
                                onclick: move |_| *ctx.focus_search_gen.write() += 1,
                                "Find a note"
                            }
                        }
                    }
                    if !recent.is_empty() {
                        section { class: "welcome-recent", aria_labelledby: "welcome-recent-title",
                            h2 { id: "welcome-recent-title", "Recently opened" }
                            ul {
                                for note in recent {
                                    {
                                        let slug = note.slug.clone();
                                        rsx! {
                                            li { key: "{note.slug}",
                                                button {
                                                    class: "welcome-note",
                                                    r#type: "button",
                                                    title: "{note.slug}",
                                                    onclick: move |_| ops.send(AppOp::OpenNote(slug.clone())),
                                                    IconFile {}
                                                    span { class: "welcome-note-text",
                                                        span { class: "welcome-note-title", "{note.title}" }
                                                        if let Some((folder, _)) = note.slug.rsplit_once('/') {
                                                            span { class: "welcome-note-location", "{folder}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
    }

    let mode = *editor_mode.read();
    let slug = active.peek().clone().unwrap_or_default();
    let note_key = format!("{slug}:{}", ctx.editor_revision.read());
    let preview_html = if mode == EditorMode::Reading {
        let body = content.read().clone();
        let srcs = image_srcs.read().clone();
        let notes = ctx.notes.read().clone();
        let resolve = |target: &str| -> Option<String> { resolve_note_slug(&notes, target) };
        let html = render_note_body(&body, &|name| srcs.get(name).cloned(), &resolve);
        #[cfg(feature = "desktop")]
        let html = {
            use cobblestone_plugin_host::SharedPluginHost;
            if let Some(host) = try_use_context::<SharedPluginHost>()
                && let Ok(host) = host.lock()
            {
                let slug = active.peek().clone().unwrap_or_default();
                let path = if slug.ends_with(".md") {
                    slug
                } else {
                    format!("{slug}.md")
                };
                host.process_html(&html, &path)
            } else {
                html
            }
        };
        html
    } else {
        String::new()
    };

    rsx! {
        div { class: "editor-container", id: "editor-area",
            div { class: "toolbar",
                input {
                    id: "note-title",
                    class: "note-title-input",
                    r#type: "text",
                    placeholder: "Note title",
                    autocomplete: "off",
                    value: "{title}",
                    readonly: !mode.is_editing(),
                    oninput: move |e| {
                        title.set(e.value());
                        mark_dirty(ctx, ops);
                    },
                }
                div { class: "toolbar-actions",
                    span {
                        class: "mode-badge",
                        title: "Editor mode",
                        "{mode.label()}"
                    }
                    span {
                        class: if *is_dirty.read() {
                            "edit-indicator unsaved"
                        } else {
                            "edit-indicator saved"
                        },
                        id: "save-indicator",
                        title: if *is_dirty.read() { "Saving…" } else { "Saved" },
                        "aria-label": if *is_dirty.read() { "Saving…" } else { "Saved" },
                    }
                    div { class: "note-menu-wrap",
                        button {
                            id: "note-menu-btn",
                            class: if *note_menu_open.read() { "icon-btn active" } else { "icon-btn" },
                            r#type: "button",
                            title: "Note options",
                            aria_label: "Note options",
                            onclick: move |e| {
                                e.stop_propagation();
                                let open = *note_menu_open.peek();
                                note_menu_open.set(!open);
                            },
                            "⋯"
                        }
                        if *note_menu_open.read() {
                            div { class: "note-menu",
                                button {
                                    class: "note-menu-item",
                                    r#type: "button",
                                    onclick: move |_| ops.send(AppOp::ToggleReading),
                                    if mode.is_editing() { "Reading view" } else { "Edit note" }
                                }
                            }
                        }
                    }
                    button {
                        id: "theme-btn",
                        class: "icon-btn",
                        r#type: "button",
                        title: "Toggle theme",
                        aria_label: "Toggle theme",
                        onclick: move |_| toggle_theme(ctx),
                        IconSunMoon {}
                    }
                    button {
                        id: "panel-btn",
                        class: if *panel_open.read() {
                            "icon-btn active"
                        } else {
                            "icon-btn"
                        },
                        r#type: "button",
                        title: "Note info",
                        aria_label: "Toggle note info",
                        aria_pressed: (*panel_open.read()).to_string(),
                        onclick: move |_| {
                            let next = !*panel_open.peek();
                            panel_open.set(next);
                        },
                        IconPanelRight {}
                    }
                }
            }
            div { class: "editor-wrap",
                match mode {
                    EditorMode::LivePreview => rsx! {
                        MarkdownEditor { key: "{note_key}", image_srcs: image_srcs }
                    },
                    EditorMode::Source => match *codemirror_ready.read() {
                        Some(true) => rsx! {
                            CodeMirrorEditor {
                                key: "{note_key}",
                                value: content,
                                preferences: ctx.editor_preferences,
                                insert: ctx.editor_insert,
                                on_change: move |text| {
                                    content.set(text);
                                    mark_dirty(ctx, ops);
                                },
                            }
                        },
                        Some(false) => rsx! {
                            textarea {
                                class: "source-editor",
                                aria_label: "Markdown source",
                                value: "{content}",
                                oninput: move |e| {
                                    content.set(e.value());
                                    mark_dirty(ctx, ops);
                                },
                            }
                        },
                        None => rsx! { p { class: "modal-hint", "Loading editor…" } },
                    },
                    EditorMode::Reading => rsx! {
                        div {
                            class: "markdown-body reading-view",
                            id: "reading-view",
                            dangerous_inner_html: "{preview_html}",
                        }
                    },
                }
            }
            div { class: "status-bar",
                span { title: "{project_label}", "{project_label}" }
                span { class: "status-bar-mode", "{mode.label()}" }
            }
        }
    }
}

fn resolve_note_slug(notes: &[NoteInfo], target: &str) -> Option<String> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    notes
        .iter()
        .find(|note| note.slug.eq_ignore_ascii_case(target))
        .or_else(|| {
            notes
                .iter()
                .find(|note| note.title.eq_ignore_ascii_case(target))
        })
        .or_else(|| {
            notes.iter().find(|note| {
                note.slug
                    .rsplit('/')
                    .next()
                    .is_some_and(|leaf| leaf.eq_ignore_ascii_case(target))
            })
        })
        .map(|note| note.slug.clone())
}
