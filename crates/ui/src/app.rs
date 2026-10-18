// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::backend::SharedBackend;
use crate::components::{ContextMenu, DragPreview, EditorArea, Modals, RightPanel, Sidebar};
use crate::ops::{AppOp, spawn_ops};
use crate::state::AppCtx;

const CSS: Asset = asset!("/assets/main.css");

/// Root component — follows the Dioxus 0.7 app pattern:
/// signals + context providers + coroutine for serialized side effects.
#[component]
pub fn App() -> Element {
    let backend = use_context::<SharedBackend>();
    let mut ctx = AppCtx::provide();
    let ops = spawn_ops(ctx, backend);
    use_context_provider(|| ops);

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
