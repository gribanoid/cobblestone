// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::actions::handle_context_action;
use crate::state::AppCtx;

#[component]
pub fn ContextMenu() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let menu = ctx.context_menu;

    let Some(state) = menu.read().clone() else {
        return rsx! {};
    };

    rsx! {
        div {
            class: "tree-context-menu",
            style: "left: {state.x}px; top: {state.y}px;",
            onmounted: move |_| {},
            for (i, item) in state.items.iter().enumerate() {
                {
                    let action = item.action.clone();
                    let label = item.label.clone();
                    let danger = item.danger;
                    rsx! {
                        button {
                            key: "{i}",
                            class: if danger {
                                "tree-context-item danger"
                            } else {
                                "tree-context-item"
                            },
                            r#type: "button",
                            onclick: move |_| handle_context_action(ctx, action.clone()),
                            "{label}"
                        }
                    }
                }
            }
        }
        // Click-away catcher
        div {
            style: "position:fixed;inset:0;z-index:149;",
            onclick: move |_| ctx.context_menu.set(None),
            oncontextmenu: move |e| {
                e.prevent_default();
                ctx.context_menu.set(None);
            },
        }
    }
}
