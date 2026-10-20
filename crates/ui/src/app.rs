// SPDX-License-Identifier: MIT

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::actions::{attach_image, AttachTarget};
use crate::backend::SharedBackend;
use crate::components::{ContextMenu, DragPreview, EditorArea, Modals, RightPanel, Sidebar};
use crate::ops::{AppOp, spawn_ops};
use crate::state::AppCtx;
use crate::util::decode_base64;

const CSS: Asset = asset!("/assets/main.css");

#[derive(Deserialize)]
struct PasteImageReq {
    /// Set when dropped onto a sidebar folder (`""` = vault root).
    folder: Option<String>,
    filename: Option<String>,
    data_base64: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct PasteImageReply {
    embed: String,
}

/// Root component — follows the Dioxus 0.7 app pattern:
/// signals + context providers + coroutine for serialized side effects.
#[component]
pub fn App() -> Element {
    let backend = use_context::<SharedBackend>();
    let mut ctx = AppCtx::provide();
    let ops = spawn_ops(ctx, backend.clone());
    use_context_provider(|| ops);

    // Obsidian-style image paste / drag-drop: save to attachments/, insert `![[…]]`.
    use_hook(move || {
        let backend = backend.clone();
        let mut ctx = ctx;
        spawn(async move {
            let mut eval = document::eval(include_str!("../assets/paste_image.js"));
            loop {
                let Ok(req) = eval.recv::<PasteImageReq>().await else {
                    break;
                };
                if let Some(err) = req.error {
                    ctx.show_error(format!("Attach image: {err}"));
                    continue;
                }
                let (Some(filename), Some(b64)) = (req.filename, req.data_base64) else {
                    continue;
                };
                let target = match req.folder {
                    Some(f) => AttachTarget::Folder(f),
                    None => AttachTarget::ActiveNote,
                };
                match decode_base64(&b64) {
                    Ok(bytes) => attach_image(ctx, backend.clone(), target, filename, bytes).await,
                    Err(e) => ctx.show_error(format!("Attach image: {e}")),
                }
                let _ = eval.send(PasteImageReply {
                    embed: String::new(),
                });
            }
        });
    });

    // Initial vault load
    use_effect(move || {
        ops.send(AppOp::ReloadTree);
    });

    // Auto-dismiss error toast
    use_effect(move || {
        if ctx.error.read().is_some() {
            spawn(async move {
                crate::timeutil::delay_ms(5000).await;
                ctx.error.set(None);
            });
        }
    });

    let theme = ctx.theme;
    let mut error = ctx.error;

    use_effect(move || {
        set_document_theme(theme.read().as_str());
    });

    rsx! {
        document::Stylesheet { href: CSS }
        document::Title { "Cobblestone" }
        div {
            class: "app",
            "data-theme": "{theme}",
            tabindex: "0",
            onkeydown: move |e| {
                if e.data().key() == Key::Escape {
                    ctx.dismiss_overlays();
                }
                let m = e.modifiers();
                if (m.meta() || m.ctrl()) && e.code() == Code::KeyS {
                    e.prevent_default();
                    ops.send(AppOp::Save);
                }
            },
            Sidebar {}
            main { class: "content",
                EditorArea {}
            }
            RightPanel {}
            Modals {}
            ContextMenu {}
            DragPreview {}
            if let Some(msg) = error.read().clone() {
                div {
                    class: "error-toast",
                    id: "error-toast",
                    onclick: move |_| error.set(None),
                    "⚠ {msg}"
                }
            }
        }
    }
}

fn set_document_theme(theme: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(el) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = el.set_attribute("data-theme", theme);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = theme;
    }
}
