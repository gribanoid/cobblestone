// SPDX-License-Identifier: MIT

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::actions::{AttachTarget, attach_image};
use crate::backend::SharedBackend;
use crate::components::{ContextMenu, DragPreview, EditorArea, Modals, RightPanel, Sidebar};
use crate::ops::{AppOp, spawn_ops};
use crate::state::{AppCtx, AppIcon, EditorPreferences};
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

#[derive(Deserialize)]
struct WikilinkNavReq {
    slug: String,
}

#[derive(Serialize)]
struct WikilinkNavAck {
    ok: bool,
}

/// Root component — follows the Dioxus 0.7 app pattern:
/// signals + context providers + coroutine for serialized side effects.
#[component]
pub fn App() -> Element {
    let backend = use_context::<SharedBackend>();
    let mut ctx = AppCtx::provide();
    let ops = spawn_ops(ctx, backend.clone());
    use_context_provider(|| ops);

    // vault-style image paste / drag-drop: save to attachments/, insert `![[…]]`.
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

    // Reading-view [[wikilink]] clicks → open note.
    use_hook(move || {
        let ops = ops;
        spawn(async move {
            let mut eval = document::eval(include_str!("../assets/wikilink_nav.js"));
            loop {
                let Ok(req) = eval.recv::<WikilinkNavReq>().await else {
                    break;
                };
                if !req.slug.is_empty() {
                    ops.send(AppOp::OpenNote(req.slug));
                }
                let _ = eval.send(WikilinkNavAck { ok: true });
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

    // Auto-dismiss notice toast
    use_effect(move || {
        if ctx.notice.read().is_some() {
            spawn(async move {
                crate::timeutil::delay_ms(3500).await;
                ctx.notice.set(None);
            });
        }
    });

    let theme = ctx.theme;
    let mut error = ctx.error;
    let mut notice = ctx.notice;

    // Restore desktop theme from webview localStorage once, then keep html/body synced.
    use_hook(move || {
        let mut theme_sig = theme;
        spawn(async move {
            let mut eval = document::eval(
                r#"
                let t = 'dark';
                try { t = localStorage.getItem('cb-theme') || 'dark'; } catch (_) {}
                document.documentElement.setAttribute('data-theme', t);
                if (document.body) document.body.setAttribute('data-theme', t);
                dioxus.send(t);
                "#,
            );
            if let Ok(saved) = eval.recv::<String>().await {
                let parsed = crate::theme::Theme::parse(&saved);
                if parsed != *theme_sig.peek() {
                    theme_sig.set(parsed);
                }
            }
        });
    });

    use_hook(move || {
        spawn(async move {
            let stored = document::eval(
                r#"try { return JSON.parse(localStorage.getItem('cb-editor')); } catch (_) { return null; }"#,
            )
            .join::<Option<EditorPreferences>>()
            .await;
            if let Ok(Some(preferences)) = stored {
                preferences.apply(ctx);
            }
        });
    });

    use_effect(move || {
        set_document_theme(theme.read().as_str());
    });

    use_hook(move || {
        spawn(async move {
            let stored = document::eval(
                r#"try { return JSON.parse(localStorage.getItem('cb-app-icon')); } catch (_) { return null; }"#,
            )
            .join::<Option<AppIcon>>()
            .await;
            if let Ok(Some(icon)) = stored
                && ctx.app_icon.peek().is_none()
            {
                ctx.app_icon.set(Some(icon));
            }
        });
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
                    ctx.note_menu_open.set(false);
                    ctx.dismiss_overlays();
                }
                let m = e.modifiers();
                if (m.meta() || m.ctrl()) && e.code() == Code::KeyS {
                    e.prevent_default();
                    ops.send(AppOp::Save);
                }
                // ⌘E / Ctrl+E — toggle Reading ↔ last editing mode.
                if (m.meta() || m.ctrl()) && e.code() == Code::KeyE {
                    e.prevent_default();
                    ops.send(AppOp::ToggleReading);
                }
                // ⌘P — plugin command palette.
                if (m.meta() || m.ctrl()) && e.code() == Code::KeyP {
                    e.prevent_default();
                    ctx.modal.set(crate::state::ModalState::CommandPalette);
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
            if let Some(msg) = notice.read().clone() {
                div {
                    class: "notice-toast",
                    id: "notice-toast",
                    onclick: move |_| notice.set(None),
                    "{msg}"
                }
            }
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
    // Keep html/body in sync so inherited `color` / `background` match `.app`.
    let theme = theme.to_string();
    spawn(async move {
        let mut eval = document::eval(
            r#"
            const t = await dioxus.recv();
            document.documentElement.setAttribute('data-theme', t);
            if (document.body) document.body.setAttribute('data-theme', t);
            try { localStorage.setItem('cb-theme', t); } catch (_) {}
            true
            "#,
        );
        let _ = eval.send(theme);
        let _ = eval.recv::<bool>().await;
    });
}
