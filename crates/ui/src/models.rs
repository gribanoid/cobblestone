// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteInfo {
    pub slug: String,
    pub title: String,
    pub created: String,
    pub modified: String,
    pub size: u64,
    pub preview: String,
    pub tags: Vec<String>,
}

/// One row in the sidebar search list (notes and folders).
#[derive(Debug, Clone, PartialEq)]
pub enum SearchHit {
    Note(NoteInfo),
    Folder { path: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteContent {
    pub slug: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkedNote {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NoteGraph {
    pub outgoing: Vec<LinkedNote>,
    pub backlinks: Vec<LinkedNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VaultNode {
    Folder {
        name: String,
        path: String,
        children: Vec<VaultNode>,
    },
    Note {
        slug: String,
        title: String,
        created: String,
        modified: String,
        size: u64,
        preview: String,
        tags: Vec<String>,
    },
}

#[cfg(feature = "desktop")]
impl From<&cobblestone_core::Note> for NoteInfo {
    fn from(n: &cobblestone_core::Note) -> Self {
        Self {
            slug: n.name.clone(),
            title: n.title.clone(),
            created: n.created.clone(),
            modified: n.modified.clone(),
            size: n.size,
            preview: n.preview.clone(),
            tags: n.tags.clone(),
        }
    }
}

#[cfg(feature = "desktop")]
impl From<cobblestone_core::VaultNode> for VaultNode {
    fn from(n: cobblestone_core::VaultNode) -> Self {
        match n {
            cobblestone_core::VaultNode::Folder {
                name,
                path,
                children,
            } => Self::Folder {
                name,
                path,
                children: children.into_iter().map(Self::from).collect(),
            },
            cobblestone_core::VaultNode::Note {
                slug,
                title,
                created,
                modified,
                size,
                preview,
                tags,
            } => Self::Note {
                slug,
                title,
                created,
                modified,
                size,
                preview,
                tags,
            },
        }
    }
}

#[cfg(feature = "desktop")]
impl From<cobblestone_core::NoteGraph> for NoteGraph {
    fn from(g: cobblestone_core::NoteGraph) -> Self {
        Self {
            outgoing: g
                .outgoing
                .into_iter()
                .map(|l| LinkedNote {
                    slug: l.slug,
                    title: l.title,
                })
                .collect(),
            backlinks: g
                .backlinks
                .into_iter()
                .map(|l| LinkedNote {
                    slug: l.slug,
                    title: l.title,
                })
                .collect(),
        }
    }
}
