// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::actions::{mark_dirty, toggle_theme};
use crate::backend::SharedBackend;
use crate::components::codemirror::{CodeMirrorEditor, use_codemirror};
use crate::components::md_editor::MarkdownEditor;
use crate::icons::{IconPanelRight, IconSunMoon};
use crate::markdown::{embed_image_names, render_markdown_preview_with_links};
use crate::models::NoteInfo;
use crate::ops::{AppOp, Ops};
use crate::state::{AppCtx, EditorMode};

const LOGO: Asset = asset!("/assets/cobblestone.png");

#[component]
pub fn EditorArea() -> Element {
    let ctx = use_context::<AppCtx>();
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

    use_effect(move || {
        let _slug = active.read().clone();
        image_srcs.write().clear();
    });

    use_effect(move || {
        let body = content.read().clone();
        let names = embed_image_names(&body);
        if names.is_empty() {
            return;
        }
        let backend = backend.clone();
        spawn(async move {
            for name in names {
                if image_srcs.peek().contains_key(&name) {
                    continue;
                }
                match backend.attachment_src(&name).await {
                    Ok(src) => {
                        image_srcs.write().insert(name, src);
                    }
                    Err(e) => {
                        eprintln!("attachment src '{name}': {e}");
                    }
                }
            }
        });
    });

    if active.read().is_none() {
        return rsx! {
            div { class: "welcome", id: "welcome",
                div { class: "logo-big",
                    img {
                        class: "app-icon app-icon-large",
                        src: LOGO,
                        alt: "",
                        width: "64",
                        height: "64",
                    }
                }
                h2 { "Cobblestone" }
                p { "Write anything. Store everything. Own it all." }
                span { class: "welcome-hint",
                    "Use the toolbar icons to create notes and folders"
                }
            }
        };
    }

    let mode = *editor_mode.read();
    let slug = active.peek().clone().unwrap_or_default();
    let note_key = format!("{slug}:{}", ctx.editor_revision.read());
    let preview_html = if mode == EditorMode::Reading {
        let body = content.read().clone();
        let t = title.read().clone();
        let srcs = image_srcs.read().clone();
        let notes = ctx.notes.read().clone();
        let resolve = |target: &str| -> Option<String> { resolve_note_slug(&notes, target) };
        let html = render_markdown_preview_with_links(
            &body,
            &t,
            &|name| srcs.get(name).cloned(),
            &resolve,
        );
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
                span { "{project_label}" }
                span { class: "status-bar-mode", "{mode.label()} · ⌘E reading · ⌘P commands" }
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
