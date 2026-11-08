// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::document::Eval;
use dioxus::prelude::*;
use serde::Serialize;

use crate::state::EditorPreferences;

const BRIDGE: &str = include_str!("codemirror_bridge.js");

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub fn use_codemirror() -> Signal<Option<bool>> {
    let mut ready = use_signal(|| None);
    use_hook(move || {
        spawn(async move {
            let loaded = document::eval(concat!(
                include_str!("../../codemirror/editor.bundle.js"),
                "\nreturn true;"
            ))
            .join::<bool>()
            .await
            .unwrap_or(false);
            ready.set(Some(loaded));
        });
    });
    ready
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Command {
    Configure(EditorPreferences),
    Insert(String),
    Destroy,
}

#[component]
pub fn CodeMirrorEditor(
    value: Signal<String>,
    preferences: Signal<EditorPreferences>,
    mut insert: Signal<Option<String>>,
    on_change: EventHandler<String>,
) -> Element {
    let id = use_hook(|| format!("codemirror-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)));
    let mut bridge = use_signal(|| None::<Eval>);
    let mut failed = use_signal(|| false);

    use_effect(move || {
        let settings = *preferences.read();
        let pending = insert.read().clone();
        let Some(eval) = *bridge.read() else {
            return;
        };
        let _ = eval.send(Command::Configure(settings));
        if let Some(text) = pending
            && eval.send(Command::Insert(text)).is_ok()
        {
            insert.set(None);
        }
    });

    use_drop(move || {
        if let Ok(state) = bridge.try_peek()
            && let Some(eval) = *state
        {
            let _ = eval.send(Command::Destroy);
        }
    });

    let initial = value.peek().clone();
    if *failed.read() {
        return rsx! {
            textarea {
                class: "source-editor",
                aria_label: "Markdown source",
                value: "{initial}",
                oninput: move |event| on_change.call(event.value()),
            }
        };
    }

    rsx! {
        div {
            id: "{id}",
            class: "cb-codemirror",
            onmounted: move |_| {
                let eval = document::eval(BRIDGE);
                if eval.send((&id, *preferences.peek())).is_err() {
                    failed.set(true);
                    return;
                }
                bridge.set(Some(eval));
                spawn(async move {
                    if eval.join::<()>().await.is_err() {
                        failed.set(true);
                    }
                });
            },
            // Native input events carry edits through the same event queue as save
            // and navigation shortcuts on both Dioxus web and desktop.
            textarea {
                hidden: true,
                tabindex: "-1",
                aria_hidden: "true",
                value: "{initial}",
                oninput: move |event| on_change.call(event.value()),
            }
            div { class: "cb-codemirror-surface" }
        }
    }
}
