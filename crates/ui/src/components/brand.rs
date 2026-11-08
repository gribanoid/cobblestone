// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::state::{AppCtx, AppIcon};

const CLASSIC: Asset = asset!("/assets/cobblestone.png");
const STONE: Asset = asset!("/assets/cobblestone-stone.png");

#[component]
pub(super) fn BrandIcon() -> Element {
    let ctx = use_context::<AppCtx>();
    match ctx.app_icon.read().unwrap_or_default() {
        AppIcon::Classic => rsx! {
            img { class: "app-icon app-icon-classic", src: CLASSIC, alt: "" }
        },
        AppIcon::Cobblestones => rsx! {
            img { class: "app-icon", src: STONE, alt: "" }
        },
    }
}
