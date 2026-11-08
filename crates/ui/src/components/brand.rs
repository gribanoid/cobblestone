// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::state::{AppCtx, AppIcon};

const CLASSIC: Asset = asset!("/assets/cobblestone.png");
const STONE: Asset = asset!("/assets/cobblestone-stone.png");

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
            img { class, src: STONE, alt: "" }
        },
    }
}
