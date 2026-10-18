// SPDX-License-Identifier: MIT

use dioxus::prelude::*;

use crate::models::NoteGraph;
use crate::ops::{AppOp, Ops};
use crate::state::AppCtx;
use crate::util::note_parent_folder;

#[component]
pub fn RightPanel() -> Element {
    let ctx = use_context::<AppCtx>();
    let ops = use_context::<Ops>();
    let active = ctx.active_slug;
    let notes = ctx.notes;
    let graph = ctx.graph;

    let Some(slug) = active.read().clone() else {
        return rsx! {
            aside { class: "right-panel", id: "right-panel",
                div { class: "panel-content", id: "panel-content",
                    div { class: "panel-empty",
                        h3 { "Note info" }
                        p { "Open a note to see tags, metadata, wikilinks, and backlinks." }
                    }
                }
            }
        };
    };

    let info = notes.read().iter().find(|n| n.slug == slug).cloned();
    let g: NoteGraph = graph.read().clone().unwrap_or_default();
    let show_move = note_parent_folder(&slug).is_some();

    rsx! {
        aside { class: "right-panel", id: "right-panel",
            div { class: "panel-content", id: "panel-content",
                section { class: "panel-section",
                    h3 { "Metadata" }
                    dl { class: "meta-list",
                        div {
                            dt { "Created" }
                            dd { "{info.as_ref().map(|i| i.created.as_str()).unwrap_or(\"\")}" }
                        }
                        div {
                            dt { "Modified" }
                            dd { "{info.as_ref().map(|i| i.modified.as_str()).unwrap_or(\"\")}" }
                        }
                        div {
                            dt { "Size" }
                            dd { "{info.as_ref().map(|i| i.size).unwrap_or(0)} B" }
                        }
                        div {
                            dt { "Path" }
                            dd { "{slug}.md" }
                        }
                    }
                }
                section { class: "panel-section",
                    h3 { "Tags" }
                    if info.as_ref().is_some_and(|i| !i.tags.is_empty()) {
                        div { class: "panel-tags",
                            for tag in info.as_ref().map(|i| i.tags.clone()).unwrap_or_default() {
                                span { class: "tag", "#{tag}" }
                            }
                        }
                    } else {
                        p { class: "panel-muted", "No tags yet" }
                    }
                }
                LinkSection {
                    title: "Outgoing links".to_string(),
                    empty: "No wikilinks found".to_string(),
                    links: g.outgoing,
                }
                LinkSection {
                    title: "Backlinks".to_string(),
                    empty: "No backlinks yet".to_string(),
                    links: g.backlinks,
                }
                if show_move {
                    section { class: "panel-section",
                        h3 { "Move" }
                        div { class: "panel-link-list",
                            button {
                                r#type: "button",
                                class: "panel-link",
                                id: "move-note-to-root-btn",
                                onclick: move |_| {
                                    ops.send(AppOp::MoveNote {
                                        slug: slug.clone(),
                                        folder: None,
                                    });
                                },
                                "Move note to root"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LinkSection(title: String, empty: String, links: Vec<crate::models::LinkedNote>) -> Element {
    let ops = use_context::<Ops>();
    rsx! {
        section { class: "panel-section",
            h3 { "{title}" }
            if links.is_empty() {
                p { class: "panel-muted", "{empty}" }
            } else {
                div { class: "panel-link-list",
                    for link in links.iter() {
                        {
                            let slug = link.slug.clone();
                            let title = link.title.clone();
                            rsx! {
                                button {
                                    r#type: "button",
                                    class: "panel-link",
                                    "data-slug": "{slug}",
                                    onclick: move |_| ops.send(AppOp::OpenNote(slug.clone())),
                                    "{title}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
