// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::actions::{mark_dirty, toggle_theme};
use crate::backend::SharedBackend;
use crate::components::md_editor::MarkdownEditor;
use crate::icons::{IconPanelRight, IconSunMoon};
use crate::markdown::embed_image_names;
use crate::ops::Ops;
use crate::state::AppCtx;

const LOGO: Asset = asset!("/assets/cobblestone.png");

#[component]
pub fn EditorArea() -> Element {
    let ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let backend = use_context::<SharedBackend>();
    let active = ctx.active_slug;
    let mut title = ctx.active_title;
    let content = ctx.active_content;
    let is_dirty = ctx.is_dirty;
    let mut panel_open = ctx.panel_open;

    // Attachment name → browser src (data URL or /api/attachments/…).
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
                    oninput: move |e| {
                        title.set(e.value());
                        mark_dirty(ctx, ops);
                    },
                }
                div { class: "toolbar-actions",
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
                MarkdownEditor { image_srcs: image_srcs }
            }
        }
    }
}
