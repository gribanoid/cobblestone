// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::state::{AppCtx, AppIcon};

const CLASSIC: Asset = asset!("/assets/cobblestone.png");

#[component]
pub(super) fn BrandIcon(#[props(default)] large: bool) -> Element {
    let ctx = use_context::<AppCtx>();
    let class = if large {
        "app-icon app-icon-large"
    } else {
        "app-icon"
    };
    match ctx.app_icon.read().unwrap_or_default() {
        AppIcon::Classic => rsx! {
            img { class: "{class} app-icon-classic", src: CLASSIC, alt: "" }
        },
        AppIcon::Cobblestones => rsx! {
            svg {
                class,
                view_box: "0 0 48 48",
                "aria-hidden": "true",
                rect { width: "48", height: "48", rx: "12", fill: "var(--surface2)" }
                path { d: "M12 12h13l3 4-2 8H10l-2-6z", fill: "var(--accent)" }
                path { d: "M32 12h5l4 5-1 9h-9l-2-5z", fill: "var(--subtext)" }
                path { d: "M11 28h26l2 5-4 4H14l-5-4z", fill: "var(--accent)", opacity: ".62" }
            }
        },
    }
}
