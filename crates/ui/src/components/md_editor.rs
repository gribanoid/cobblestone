// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::actions::{attach_image, mark_dirty, AttachTarget};
use crate::backend::SharedBackend;
use crate::ops::Ops;
use crate::state::AppCtx;

/// Cobblestone note editor — thin wrapper around `cobblestone-editor`.
#[component]
pub fn MarkdownEditor(image_srcs: Signal<HashMap<String, String>>) -> Element {
    let ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let backend = use_context::<SharedBackend>();
    let active = ctx.active_slug;
    let mut content = ctx.active_content;

    let mut editor_value = use_signal(|| content.peek().clone());
    let mut skip_dirty = use_signal(|| true);

    // Note switch: load body into the editor without marking dirty.
    use_effect(move || {
        let _slug = active.read().clone();
        skip_dirty.set(true);
        editor_value.set(content.peek().clone());
    });

    // Editor → note body (+ autosave dirty bit).
    use_effect(move || {
        let raw = editor_value.read().clone();
        if raw == *content.peek() {
            if *skip_dirty.peek() {
                skip_dirty.set(false);
            }
            return;
        }
        content.set(raw);
        if *skip_dirty.peek() {
            skip_dirty.set(false);
            return;
        }
        mark_dirty(ctx, ops);
    });

    // External body updates (image paste) → editor, without a dirty loop.
    use_effect(move || {
        let body = content.read().clone();
        if body != *editor_value.peek() {
            skip_dirty.set(true);
            editor_value.set(body);
        }
    });

    let mount_key = active
        .read()
        .clone()
        .unwrap_or_else(|| "__none__".into());

    rsx! {
        div {
            class: "cb-md-host",
            key: "{mount_key}",
            cobblestone_editor::MarkdownEditor {
                value: editor_value,
                image_srcs: image_srcs,
                insert_embed: ctx.editor_insert,
                on_paste_image: move |img: cobblestone_editor::PastedImage| {
                    let backend = backend.clone();
                    spawn(async move {
                        attach_image(
                            ctx,
                            backend,
                            AttachTarget::ActiveNote,
                            img.filename.unwrap_or_default(),
                            img.bytes,
                        )
                        .await;
                    });
                },
            }
        }
    }
}
