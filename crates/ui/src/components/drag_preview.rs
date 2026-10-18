// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::icons::{IconFile, IconFolder};
use crate::state::{AppCtx, DragPayload};
use crate::util::can_drop_note_or_folder;

/// Floating ghost + destination hint while dragging tree items.
#[component]
pub fn DragPreview() -> Element {
    let ctx = use_context::<AppCtx>();
    let Some(drag) = ctx.drag.read().clone() else {
        return rsx! {};
    };

    let can_drop = match &drag.over_folder {
        Some(dest) => match &drag.payload {
            DragPayload::Note { slug } => {
                can_drop_note_or_folder(true, Some(slug), None, dest.as_deref())
            }
            DragPayload::Folder { path } => {
                can_drop_note_or_folder(false, None, Some(path), dest.as_deref())
            }
        },
        None => false,
    };

    let hint = drag.destination_hint();
    let is_folder = matches!(drag.payload, DragPayload::Folder { .. });
    // Offset so the card sits under/right of the cursor.
    let left = drag.cursor_x + 14.0;
    let top = drag.cursor_y + 10.0;

    rsx! {
        div {
            class: if can_drop {
                "drag-preview"
            } else if drag.over_folder.is_some() {
                "drag-preview drag-preview-blocked"
            } else {
                "drag-preview"
            },
            style: "left: {left}px; top: {top}px;",
            div { class: "drag-preview-row",
                span { class: "drag-preview-icon",
                    if is_folder {
                        IconFolder {}
                    } else {
                        IconFile {}
                    }
                }
                span { class: "drag-preview-label", "{drag.label}" }
            }
            if can_drop {
                div { class: "drag-preview-hint", "{hint}" }
            } else if drag.over_folder.is_some() {
                div { class: "drag-preview-hint", "Can't drop here" }
            }
        }
    }
}

/// Hide the OS/browser default drag ghost when the platform allows it (wasm).
pub fn suppress_native_drag_ghost(data: &dioxus::html::DragData) {
    let _ = data.data_transfer().set_data("text/plain", "cobblestone-dnd");
    data.data_transfer().set_effect_allowed("move");

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(evt) = data.downcast::<web_sys::DragEvent>() {
            if let Some(dt) = evt.data_transfer() {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    if let Ok(img) = doc.create_element("img") {
                        if let Ok(img) = img.dyn_into::<web_sys::HtmlImageElement>() {
                            // 1×1 transparent GIF — empty native ghost, we draw our own card.
                            img.set_src(
                                "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7",
                            );
                            dt.set_drag_image(&img, 0, 0);
                        }
                    }
                }
            }
        }
    }
}
