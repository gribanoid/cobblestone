// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

/// New-note affordance: rounded square with a pen (Lucide `square-pen`).
#[component]
pub fn IconSquarePen() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" }
            path { d: "M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z" }
        }
    }
}

#[component]
pub fn IconFolderPlus() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 10v6" }
            path { d: "M9 13h6" }
            path { d: "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9l-.81-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
        }
    }
}

#[component]
pub fn IconSunMoon() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 2v2" }
            path { d: "m4.93 4.93 1.41 1.41" }
            path { d: "M20 12h2" }
            path { d: "m19.07 4.93-1.41 1.41" }
            path { d: "M12 22v-2" }
            path { d: "m19.07 19.07-1.41-1.41" }
            path { d: "M2 12h2" }
            path { d: "m4.93 19.07 1.41-1.41" }
            circle { cx: "12", cy: "12", r: "4" }
        }
    }
}

#[component]
pub fn IconChevron() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "14",
            height: "14",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "m9 18 6-6-6-6" }
        }
    }
}

#[component]
pub fn IconFolder() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "14",
            height: "14",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9l-.81-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" }
        }
    }
}

#[component]
pub fn IconFile() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "13",
            height: "13",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" }
            path { d: "M14 2v4a2 2 0 0 0 2 2h4" }
        }
    }
}

#[component]
pub fn IconArrowUpDown() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "m21 16-4 4-4-4" }
            path { d: "M17 20V4" }
            path { d: "m3 8 4-4 4 4" }
            path { d: "M7 4v16" }
        }
    }
}

#[component]
pub fn IconChevronsUpDown() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "m7 15 5 5 5-5" }
            path { d: "m7 9 5-5 5 5" }
        }
    }
}

#[component]
pub fn IconCheck() -> Element {
    rsx! {
        svg {
            class: "lucide",
            xmlns: "http://www.w3.org/2000/svg",
            width: "14",
            height: "14",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M20 6 9 17l-5-5" }
        }
    }
}
