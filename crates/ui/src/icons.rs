// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

#[component]
pub fn IconBookOpen() -> Element {
    rsx! {
        svg {
            class: "lucide",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M12 6c-2-2-5-3-9-2v15c4-1 7 0 9 2 2-2 5-3 9-2V4c-4-1-7 0-9 2v15" }
        }
    }
}

#[component]
pub fn IconPencil() -> Element {
    rsx! {
        svg {
            class: "lucide",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "m4 15-1 6 6-1L21 8a2.8 2.8 0 0 0-4-4Z M15 6l4 4" }
        }
    }
}

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

/// Lucide `image` — attachment rows in the vault tree.
#[component]
pub fn IconImage() -> Element {
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
            rect { width: "18", height: "18", x: "3", y: "3", rx: "2", ry: "2" }
            circle { cx: "9", cy: "9", r: "2" }
            path { d: "m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" }
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

/// Lucide `panel-right` — toggle note metadata sidebar.
#[component]
pub fn IconPanelRight() -> Element {
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
            rect { width: "18", height: "18", x: "3", y: "3", rx: "2" }
            path { d: "M15 3v18" }
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

/// Lucide `settings` — gear for app preferences.
#[component]
pub fn IconSettings() -> Element {
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
            path { d: "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" }
            circle { cx: "12", cy: "12", r: "3" }
        }
    }
}
