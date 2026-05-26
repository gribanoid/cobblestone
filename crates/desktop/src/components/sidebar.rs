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

use crate::tauri::NoteInfo;

#[component]
pub fn Sidebar(
    notes:         Signal<Vec<NoteInfo>>,
    active_slug:   Signal<Option<String>>,
    on_select:     Callback<String>,
    on_new:        Callback<String>,
    on_search:     Callback<String>,
) -> impl IntoView {
    let search_val = RwSignal::new(String::new());

    // Filtered note list (reactive on search_val + notes)
    let visible = Memo::new(move |_| {
        let q = search_val.get().to_lowercase();
        notes.get()
            .into_iter()
            .filter(|n| {
                q.is_empty()
                    || n.title.to_lowercase().contains(&q)
                    || n.preview.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    });

    let on_search_input = move |ev: web_sys::Event| {
        let val = event_target_value(&ev);
        search_val.set(val.clone());
        on_search.run(val);
    };

    let on_new_click = move |_| {
        let title = search_val.get();
        let title = if title.is_empty() {
            format!("Note {}", js_sys::Date::now() as u64)
        } else {
            title
        };
        on_new.run(title);
    };

    view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <span class="logo">"🪨 Cobblestone"</span>
            </div>

            <div class="search-wrap">
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search notes…"
                    prop:value=search_val
                    on:input=on_search_input
                />
            </div>

            <button class="new-btn" on:click=on_new_click>
                "+ New note"
            </button>

            <div class="note-list">
                {move || {
                    let items = visible.get();
                    if items.is_empty() {
                        view! {
                            <div class="empty-state">
                                "No notes yet." <br/>
                                "Click \"+ New note\" to start."
                            </div>
                        }.into_any()
                    } else {
                        items.into_iter().map(|note| {
                            let slug       = note.slug.clone();
                            let slug_click = slug.clone();
                            let is_active  = move || active_slug.get().as_deref() == Some(&slug);

                            view! {
                                <div
                                    class=move || format!("note-item{}", if is_active() { " active" } else { "" })
                                    on:click=move |_| on_select.run(slug_click.clone())
                                >
                                    <div class="note-item-title">{note.title.clone()}</div>
                                    <div class="note-item-meta">
                                        {note.modified.clone()}
                                        {if note.size > 0 {
                                            format!(" · {} B", note.size)
                                        } else { String::new() }}
                                    </div>
                                    {if note.tags.is_empty() { None } else {
                                        Some(view! {
                                            <div class="note-item-tags">
                                                {note.tags.iter().map(|t| view! {
                                                    <span class="tag">{"#"}{t.clone()}</span>
                                                }).collect_view()}
                                            </div>
                                        })
                                    }}
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>
        </aside>
    }
}
