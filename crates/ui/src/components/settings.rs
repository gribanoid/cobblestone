// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::icons::{IconFolder, IconSettings, IconSquarePen, IconSunMoon};
use crate::state::{AppCtx, AppIcon, EditorPreferences, ModalState};

const CSS: Asset = asset!("/assets/settings.css");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Editor,
    Appearance,
    Projects,
    Shortcuts,
}

impl Section {
    const ALL: [Self; 4] = [
        Self::Editor,
        Self::Appearance,
        Self::Projects,
        Self::Shortcuts,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Editor => "Editor",
            Self::Appearance => "Appearance",
            Self::Projects => "Projects",
            Self::Shortcuts => "Shortcuts",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Editor => "Choose how Markdown looks and behaves while you write.",
            Self::Appearance => "Change the look of Cobblestone.",
            Self::Projects => "Manage the local folders you use as vaults.",
            Self::Shortcuts => "Find the shortcuts for everyday actions.",
        }
    }
}

#[component]
pub(super) fn SettingsModal() -> Element {
    let mut ctx = use_context::<AppCtx>();
    let mut section = use_signal(|| Section::Editor);
    let current = *section.read();

    rsx! {
        document::Stylesheet { href: CSS }
        div { class: "modal-backdrop settings-backdrop",
            div {
                class: "settings-dialog",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "settings-title",
                header { class: "settings-header",
                    h2 { id: "settings-title", "Settings" }
                    button {
                        class: "tb-btn settings-button",
                        r#type: "button",
                        onmounted: move |event| {
                            spawn(async move { let _ = event.data().set_focus(true).await; });
                        },
                        onclick: move |_| ctx.modal.set(ModalState::None),
                        "Close"
                    }
                }
                div { class: "settings-layout",
                    nav { class: "settings-nav", aria_label: "Settings sections",
                        for entry in Section::ALL {
                            button {
                                class: "settings-nav-button",
                                r#type: "button",
                                aria_pressed: current == entry,
                                onclick: move |_| section.set(entry),
                                span { class: "settings-nav-icon", aria_hidden: "true",
                                    match entry {
                                        Section::Editor => rsx! { IconSquarePen {} },
                                        Section::Appearance => rsx! { IconSunMoon {} },
                                        Section::Projects => rsx! { IconFolder {} },
                                        Section::Shortcuts => rsx! { IconSettings {} },
                                    }
                                }
                                "{entry.title()}"
                            }
                        }
                    }
                    section { class: "settings-panel", key: "{current.title()}",
                        h3 { "{current.title()}" }
                        p { class: "settings-intro", "{current.description()}" }
                        div { class: "settings-group",
                            match current {
                                Section::Editor => rsx! { EditorSettings {} },
                                Section::Appearance => rsx! {
                                    SettingRow {
                                        title: "Color theme",
                                        description: "Apply a light or dark palette to the whole app.",
                                        button {
                                            class: "tb-btn settings-button",
                                            r#type: "button",
                                            onclick: move |_| crate::actions::toggle_theme(ctx),
                                            "Switch theme ({ctx.theme.read().as_str()})"
                                        }
                                    }
                                    SettingRow {
                                        title: "App icon",
                                        description: "Choose the icon shown on the welcome screen and in the sidebar.",
                                        label_for: "app-icon-choice",
                                        select {
                                            id: "app-icon-choice",
                                            aria_describedby: "app-icon-choice-hint",
                                            class: "settings-select",
                                            value: match ctx.app_icon.read().unwrap_or_default() {
                                                AppIcon::Cobblestones => "cobblestones",
                                                AppIcon::Classic => "classic",
                                            },
                                            onchange: move |event| {
                                                let icon = if event.value() == "classic" {
                                                    AppIcon::Classic
                                                } else {
                                                    AppIcon::Cobblestones
                                                };
                                                ctx.app_icon.set(Some(icon));
                                                icon.persist();
                                            },
                                            option { value: "cobblestones", "Cobblestones" }
                                            option { value: "classic", "Classic pixel icon" }
                                        }
                                    }
                                },
                                Section::Projects => rsx! {
                                    SettingRow {
                                        title: "Vault folders",
                                        description: "Add a local folder or switch to another project.",
                                        button {
                                            class: "tb-btn settings-button",
                                            r#type: "button",
                                            onclick: move |_| ctx.modal.set(ModalState::Projects),
                                            "Manage projects"
                                        }
                                    }
                                },
                                Section::Shortcuts => rsx! {
                                    SettingRow {
                                        title: "Toggle reading view",
                                        description: "Switch between editing and reading the current note.",
                                        kbd { "⌘ / Ctrl + E" }
                                    }
                                    SettingRow {
                                        title: "Save note",
                                        description: "Write the current changes to disk.",
                                        kbd { "⌘ / Ctrl + S" }
                                    }
                                    SettingRow {
                                        title: "Command palette",
                                        description: "Search for available commands.",
                                        kbd { "⌘ / Ctrl + P" }
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SettingRow(
    title: &'static str,
    description: &'static str,
    label_for: Option<&'static str>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "settings-row",
            div { class: "settings-copy",
                if let Some(id) = label_for {
                    label { r#for: id, class: "settings-label", "{title}" }
                } else {
                    span { class: "settings-label", "{title}" }
                }
                p { id: label_for.map(|id| format!("{id}-hint")), "{description}" }
            }
            div { class: "settings-control", {children} }
        }
    }
}

#[component]
fn EditorSettings() -> Element {
    let ctx = use_context::<AppCtx>();
    let preferences = *ctx.editor_preferences.read();
    let update = move |preferences: EditorPreferences| {
        preferences.apply(ctx);
        preferences.persist();
    };

    rsx! {
        SettingRow {
            title: "Markdown editor",
            description: "Use syntax highlighting or try the Live Preview beta.",
            label_for: "editor-engine",
            select {
                id: "editor-engine",
                aria_describedby: "editor-engine-hint",
                class: "settings-select",
                value: if preferences.live_preview { "live_preview" } else { "codemirror" },
                onchange: move |event| {
                    let mut next = *ctx.editor_preferences.peek();
                    next.live_preview = event.value() == "live_preview";
                    update(next);
                },
                option { value: "codemirror", "CodeMirror" }
                option { value: "live_preview", "Live Preview (beta)" }
            }
        }
        if !preferences.live_preview {
            SettingRow {
                title: "Line numbers",
                description: "Show the line number beside each line of Markdown.",
                label_for: "editor-line-numbers",
                input {
                    id: "editor-line-numbers",
                    aria_describedby: "editor-line-numbers-hint",
                    class: "settings-switch",
                    r#type: "checkbox",
                    role: "switch",
                    checked: preferences.line_numbers,
                    onchange: move |event| {
                        let mut next = *ctx.editor_preferences.peek();
                        next.line_numbers = event.checked();
                        update(next);
                    },
                }
            }
            SettingRow {
                title: "Wrap long lines",
                description: "Keep long lines visible without horizontal scrolling.",
                label_for: "editor-line-wrapping",
                input {
                    id: "editor-line-wrapping",
                    aria_describedby: "editor-line-wrapping-hint",
                    class: "settings-switch",
                    r#type: "checkbox",
                    role: "switch",
                    checked: preferences.line_wrapping,
                    onchange: move |event| {
                        let mut next = *ctx.editor_preferences.peek();
                        next.line_wrapping = event.checked();
                        update(next);
                    },
                }
            }
            SettingRow {
                title: "Tab width",
                description: "Set the width of a tab and the size of an indent.",
                label_for: "editor-tab-width",
                select {
                    id: "editor-tab-width",
                    aria_describedby: "editor-tab-width-hint",
                    class: "settings-select",
                    value: "{preferences.tab_width}",
                    onchange: move |event| {
                        if let Ok(tab_width) = event.parsed() {
                            let mut next = *ctx.editor_preferences.peek();
                            next.tab_width = tab_width;
                            update(next);
                        }
                    },
                    option { value: "2", "2 spaces" }
                    option { value: "4", "4 spaces" }
                    option { value: "8", "8 spaces" }
                }
            }
        }
    }
}
