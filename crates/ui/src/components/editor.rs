// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::actions::{mark_dirty, toggle_theme};
use crate::icons::IconSunMoon;
use crate::markdown::render_markdown_preview;
use crate::ops::{AppOp, Ops};
use crate::state::AppCtx;
use crate::util::strip_leading_heading;

const LOGO: Asset = asset!("/assets/cobblestone.png");

#[component]
pub fn EditorArea() -> Element {
    let ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let active = ctx.active_slug;
    let mut title = ctx.active_title;
    let mut content = ctx.active_content;
    let mut is_editing = ctx.is_editing;
    let is_dirty = ctx.is_dirty;

    // Derived preview — recomputes only when content/title change (Dioxus memo idiom).
    let preview_html = use_memo(move || render_markdown_preview(&content.read(), &title.read()));

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
                            "edit-indicator"
                        },
                        id: "save-indicator",
                        if *is_dirty.read() { "unsaved" } else { "saved" }
                    }
                    button {
                        id: "toggle-mode-btn",
                        class: "tb-btn",
                        onclick: move |_| {
                            let next = !*is_editing.peek();
                            is_editing.set(next);
                        },
                        if *is_editing.read() { "Preview" } else { "Edit" }
                    }
                    button {
                        id: "save-btn",
                        class: "tb-btn primary",
                        onclick: move |_| ops.send(AppOp::Save),
                        "Save"
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
                }
            }
            div { class: "editor-wrap",
                textarea {
                    id: "editor",
                    class: if *is_editing.read() { "editor" } else { "editor hidden" },
                    spellcheck: "false",
                    value: "{content}",
                    oninput: move |e| {
                        content.set(strip_leading_heading(&e.value()));
                        mark_dirty(ctx, ops);
                    },
                    onkeydown: move |e| {
                        if e.data().key() == Key::Tab {
                            e.prevent_default();
                            let mut c = content.peek().clone();
                            c.push_str("  ");
                            content.set(c);
                            mark_dirty(ctx, ops);
                            return;
                        }
                        let key = e.data().key();
                        let mods = e.data().modifiers();
                        let is_s = matches!(key, Key::Character(ref c) if c.eq_ignore_ascii_case("s"));
                        if is_s
                            && (mods.contains(Modifiers::CONTROL) || mods.contains(Modifiers::META))
                        {
                            e.prevent_default();
                            ops.send(AppOp::Save);
                        }
                    },
                }
                div {
                    id: "preview",
                    class: if *is_editing.read() { "preview hidden" } else { "preview" },
                    dangerous_inner_html: "{preview_html}",
                }
            }
        }
    }
}
