// SPDX-License-Identifier: GPL-3.0-or-later
//
// Cobblestone — open-source knowledge base for your private thoughts
// Copyright (C) 2026  Cobblestone Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use leptos::prelude::*;

use crate::{
    components::{
        editor::Editor,
        sidebar::Sidebar,
        toolbar::Toolbar,
    },
    tauri::{self, NoteInfo},
};

// ---------------------------------------------------------------------------
// Root App component
// ---------------------------------------------------------------------------

#[component]
pub fn App() -> impl IntoView {
    // ── Global state ────────────────────────────────────────────────────────
    let notes       = RwSignal::<Vec<NoteInfo>>::new(vec![]);
    let active_slug = RwSignal::<Option<String>>::new(None);
    let title       = RwSignal::<String>::new(String::new());
    let content     = RwSignal::<String>::new(String::new());
    let is_editing  = RwSignal::new(false);
    let is_dirty    = RwSignal::new(false);
    let error_msg   = RwSignal::<Option<String>>::new(None);

    // ── Load note list on mount ──────────────────────────────────────────────
    let load_notes = {
        move || {
            leptos::task::spawn_local(async move {
                match tauri::list_notes().await {
                    Ok(list) => notes.set(list),
                    Err(e)   => error_msg.set(Some(e)),
                }
            });
        }
    };

    // Run on mount
    Effect::new(move |_| { load_notes(); });

    // ── Select / open a note ────────────────────────────────────────────────
    let open_note = Callback::new(move |slug: String| {
        let slug_clone = slug.clone();
        leptos::task::spawn_local(async move {
            match tauri::get_note(&slug_clone).await {
                Ok(note) => {
                    active_slug.set(Some(slug_clone));
                    title.set(note.title);
                    content.set(note.content);
                    is_editing.set(false);
                    is_dirty.set(false);
                }
                Err(e) => error_msg.set(Some(e)),
            }
        });
    });

    // ── Create note ──────────────────────────────────────────────────────────
    let create_note = Callback::new(move |note_title: String| {
        leptos::task::spawn_local(async move {
            match tauri::create_note(&note_title).await {
                Ok(slug) => {
                    load_notes();
                    open_note.run(slug);
                    is_editing.set(true);
                }
                Err(e) => error_msg.set(Some(e)),
            }
        });
    });

    // ── Save note ────────────────────────────────────────────────────────────
    let save_note = Callback::new(move |()| {
        if let Some(slug) = active_slug.get() {
            let c = content.get();
            leptos::task::spawn_local(async move {
                match tauri::save_note(&slug, &c).await {
                    Ok(_) => {
                        is_dirty.set(false);
                        load_notes();
                    }
                    Err(e) => error_msg.set(Some(e)),
                }
            });
        }
    });

    // ── Delete note ──────────────────────────────────────────────────────────
    let delete_note = Callback::new(move |()| {
        if let Some(slug) = active_slug.get() {
            leptos::task::spawn_local(async move {
                match tauri::delete_note(&slug).await {
                    Ok(_) => {
                        active_slug.set(None);
                        title.set(String::new());
                        content.set(String::new());
                        is_dirty.set(false);
                        load_notes();
                    }
                    Err(e) => error_msg.set(Some(e)),
                }
            });
        }
    });

    // ── Theme toggle ─────────────────────────────────────────────────────────
    let toggle_theme = Callback::new(move |()| {
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(html) = doc.document_element() {
                    let current = html.get_attribute("data-theme").unwrap_or_default();
                    let next = if current == "dark" { "light" } else { "dark" };
                    let _ = html.set_attribute("data-theme", next);
                    // persist in localStorage
                    if let Ok(Some(storage)) = win.local_storage() {
                        let _ = storage.set_item("cb-theme", next);
                    }
                }
            }
        }
    });

    // ── Content change ───────────────────────────────────────────────────────
    let on_content_change = Callback::new(move |val: String| {
        content.set(val);
        is_dirty.set(true);
    });

    let on_title_change = Callback::new(move |val: String| {
        title.set(val);
        is_dirty.set(true);
    });

    // ── Keyboard shortcuts ────────────────────────────────────────────────────
    Effect::new(move |_| {
        use wasm_bindgen::JsCast;
        let handler = wasm_bindgen::closure::Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(
            move |e: web_sys::KeyboardEvent| {
                if (e.ctrl_key() || e.meta_key()) && e.key() == "s" {
                    e.prevent_default();
                    save_note.run(());
                }
            },
        );
        if let Some(win) = web_sys::window() {
            let _ = win.add_event_listener_with_callback(
                "keydown",
                handler.as_ref().unchecked_ref(),
            );
        }
        handler.forget();
    });

    let note_open = move || active_slug.get().is_some();

    view! {
        <div class="app">
            <Sidebar
                notes=Signal::derive(move || notes.get())
                active_slug=Signal::derive(move || active_slug.get())
                on_select=open_note
                on_new=create_note
                on_search=Callback::new(|_| {})  // handled inside Sidebar
            />

            <main class="content">
                // Welcome screen
                {move || if !note_open() {
                    view! {
                        <div class="welcome">
                            <div class="logo-big">"🪨"</div>
                            <h2>"Welcome to Cobblestone"</h2>
                            <p>"Write anything. Store everything. Own it all.<br/>\
                                Select a note or create a new one."</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div style="display:flex;flex-direction:column;flex:1;min-height:0">
                            <Toolbar
                                title=Signal::derive(move || title.get())
                                is_editing=Signal::derive(move || is_editing.get())
                                is_dirty=Signal::derive(move || is_dirty.get())
                                on_title_change=on_title_change
                                on_toggle_mode=Callback::new(move |()| is_editing.update(|v| *v = !*v))
                                on_save=save_note
                                on_delete=delete_note
                                on_theme_toggle=toggle_theme
                            />
                            <Editor
                                content=Signal::derive(move || content.get())
                                is_editing=Signal::derive(move || is_editing.get())
                                on_change=on_content_change
                            />
                        </div>
                    }.into_any()
                }}

                // Error toast
                {move || error_msg.get().map(|e| view! {
                    <div class="error-toast" on:click=move |_| error_msg.set(None)>
                        "⚠ " {e}
                    </div>
                })}
            </main>
        </div>
    }
}
