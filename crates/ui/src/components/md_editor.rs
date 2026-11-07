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
    let mount_key = ctx.active_slug.read().clone().unwrap_or_default();

    rsx! {
        div {
            class: "cb-md-host",
            key: "{mount_key}",
            cobblestone_editor::MarkdownEditor {
                value: ctx.active_content,
                on_change: move |()| mark_dirty(ctx, ops),
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
