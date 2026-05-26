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

#[component]
pub fn Toolbar(
    title:       Signal<String>,
    is_editing:  Signal<bool>,
    is_dirty:    Signal<bool>,
    on_title_change: Callback<String>,
    on_toggle_mode:  Callback<()>,
    on_save:         Callback<()>,
    on_delete:       Callback<()>,
    on_theme_toggle: Callback<()>,
) -> impl IntoView {
    let save_label = move || {
        if is_dirty.get() { "● Save" } else { "Saved" }
    };

    view! {
        <div class="toolbar">
            <input
                class="note-title-input"
                placeholder="Note title"
                prop:value=title
                on:input=move |ev| on_title_change.run(event_target_value(&ev))
            />
            <div class="toolbar-actions">
                <span class=move || format!(
                    "edit-indicator{}",
                    if is_dirty.get() { " unsaved" } else { "" }
                )>
                    {move || if is_dirty.get() { "unsaved" } else { "saved" }}
                </span>

                <button
                    class=move || format!(
                        "tb-btn{}",
                        if is_editing.get() { "" } else { " active" }
                    )
                    on:click=move |_| on_toggle_mode.run(())
                >
                    {move || if is_editing.get() { "Preview" } else { "Edit" }}
                </button>

                <button
                    class=move || format!(
                        "tb-btn primary{}",
                        if is_dirty.get() { "" } else { " dimmed" }
                    )
                    on:click=move |_| on_save.run(())
                >
                    {save_label}
                </button>

                <button class="tb-btn danger" on:click=move |_| on_delete.run(())>
                    "Delete"
                </button>

                <button class="icon-btn" on:click=move |_| on_theme_toggle.run(())>
                    "◑"
                </button>
            </div>
        </div>
    }
}
