// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::models::{NoteGraph, NoteInfo, SearchHit, VaultNode};
use crate::theme::Theme;

/// Reactive UI state shared via [`use_context`].
///
/// `SharedBackend` is injected separately with
/// [`LaunchBuilder::with_context`](dioxus::LaunchBuilder) and read via
/// [`use_context`] in components / the ops coroutine.
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub tree: Signal<Vec<VaultNode>>,
    pub notes: Signal<Vec<NoteInfo>>,
    pub search_results: Signal<Option<Vec<SearchHit>>>,
    pub search_query: Signal<String>,
    /// Bumps on every search input; async workers check before applying results.
    pub search_gen: Signal<u64>,
    pub expanded: Signal<HashSet<String>>,
    pub active_slug: Signal<Option<String>>,
    pub active_title: Signal<String>,
    pub active_content: Signal<String>,
    pub persisted_title: Signal<String>,
    pub graph: Signal<Option<NoteGraph>>,
    pub is_dirty: Signal<bool>,
    /// Bumps on every edit; autosave only commits if gen still matches.
    pub dirty_gen: Signal<u64>,
    pub theme: Signal<Theme>,
    pub app_icon: Signal<Option<AppIcon>>,
    pub error: Signal<Option<String>>,
    /// Soft plugin/host notices (toast).
    pub notice: Signal<Option<String>>,
    pub busy: Signal<bool>,
    pub modal: Signal<ModalState>,
    /// When true, right panel shows vault tasks section.
    pub tasks_panel: Signal<bool>,
    /// Bumped to focus the sidebar search input.
    pub focus_search_gen: Signal<u64>,
    pub context_menu: Signal<Option<ContextMenuState>>,
    pub drag: Signal<Option<DragState>>,
    pub tree_sort: Signal<TreeSort>,
    /// Right metadata panel (hidden by default).
    pub panel_open: Signal<bool>,
    /// Line for the editor to insert at the caret (e.g. a saved image embed).
    pub editor_insert: Signal<Option<String>>,
    /// Bumps when the active editor document is opened, cleared or replaced.
    pub editor_revision: Signal<u64>,
    /// Source / Live Preview / Reading (vault-style).
    pub editor_mode: Signal<EditorMode>,
    pub editor_preferences: Signal<EditorPreferences>,
    /// Current project / vault label (status bar / sidebar).
    pub project_label: Signal<String>,
    /// Overflow menu (⋯) open on the note toolbar.
    pub note_menu_open: Signal<bool>,
}

impl AppCtx {
    pub fn provide() -> Self {
        let ctx = Self {
            tree: use_signal(Vec::new),
            notes: use_signal(Vec::new),
            search_results: use_signal(|| None),
            search_query: use_signal(String::new),
            search_gen: use_signal(|| 0u64),
            expanded: use_signal(HashSet::new),
            active_slug: use_signal(|| None),
            active_title: use_signal(String::new),
            active_content: use_signal(String::new),
            persisted_title: use_signal(String::new),
            graph: use_signal(|| None),
            is_dirty: use_signal(|| false),
            dirty_gen: use_signal(|| 0u64),
            theme: use_signal(Theme::load),
            app_icon: use_signal(|| None),
            error: use_signal(|| None),
            notice: use_signal(|| None),
            busy: use_signal(|| false),
            modal: use_signal(|| ModalState::None),
            context_menu: use_signal(|| None),
            drag: use_signal(|| None),
            tree_sort: use_signal(|| TreeSort::NameAsc),
            panel_open: use_signal(|| false),
            tasks_panel: use_signal(|| false),
            focus_search_gen: use_signal(|| 0u64),
            editor_insert: use_signal(|| None),
            editor_revision: use_signal(|| 0),
            editor_mode: use_signal(EditorMode::default),
            editor_preferences: use_signal(EditorPreferences::default),
            project_label: use_signal(|| "Cobblestone".into()),
            note_menu_open: use_signal(|| false),
        };
        use_context_provider(|| ctx);
        ctx
    }

    pub fn show_error(&mut self, msg: impl Into<String>) {
        self.error.set(Some(msg.into()));
    }

    pub fn show_notice(&mut self, msg: impl Into<String>) {
        self.notice.set(Some(msg.into()));
    }

    pub fn dismiss_overlays(&mut self) {
        if *self.modal.peek() != ModalState::None {
            self.modal.set(ModalState::None);
        }
        if self.context_menu.peek().is_some() {
            self.context_menu.set(None);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppIcon {
    #[default]
    Cobblestones,
    Classic,
}

impl AppIcon {
    pub fn persist(self) {
        let eval = document::eval(
            r#"try { localStorage.setItem('cb-app-icon', JSON.stringify(await dioxus.recv())); } catch (_) {}"#,
        );
        let _ = eval.send(self);
    }
}

/// How the vault file tree is ordered in the sidebar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TreeSort {
    #[default]
    NameAsc,
    NameDesc,
    ModifiedNew,
    ModifiedOld,
    CreatedNew,
    CreatedOld,
}

impl TreeSort {
    pub const ALL: [Self; 6] = [
        Self::NameAsc,
        Self::NameDesc,
        Self::ModifiedNew,
        Self::ModifiedOld,
        Self::CreatedNew,
        Self::CreatedOld,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::NameAsc => "File name (A to Z)",
            Self::NameDesc => "File name (Z to A)",
            Self::ModifiedNew => "Modified time (new to old)",
            Self::ModifiedOld => "Modified time (old to new)",
            Self::CreatedNew => "Created time (new to old)",
            Self::CreatedOld => "Created time (old to new)",
        }
    }

    pub fn is_name_group(self) -> bool {
        matches!(self, Self::NameAsc | Self::NameDesc)
    }

    pub fn is_modified_group(self) -> bool {
        matches!(self, Self::ModifiedNew | Self::ModifiedOld)
    }

    pub fn is_created_group(self) -> bool {
        matches!(self, Self::CreatedNew | Self::CreatedOld)
    }
}

/// vault-style note view modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorMode {
    LivePreview,
    #[default]
    Source,
    Reading,
}

impl EditorMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::LivePreview => "Live Preview (beta)",
            Self::Source => "CodeMirror",
            Self::Reading => "Reading",
        }
    }

    pub fn is_editing(self) -> bool {
        !matches!(self, Self::Reading)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct EditorPreferences {
    pub live_preview: bool,
    pub line_numbers: bool,
    pub line_wrapping: bool,
    pub tab_width: u8,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            live_preview: false,
            line_numbers: true,
            line_wrapping: true,
            tab_width: 2,
        }
    }
}

impl EditorPreferences {
    pub fn editing_mode(self) -> EditorMode {
        if self.live_preview {
            EditorMode::LivePreview
        } else {
            EditorMode::Source
        }
    }

    pub fn apply(mut self, mut ctx: AppCtx) {
        if !matches!(self.tab_width, 2 | 4 | 8) {
            self.tab_width = Self::default().tab_width;
        }
        ctx.editor_preferences.set(self);
        if ctx.editor_mode.peek().is_editing() {
            ctx.editor_mode.set(self.editing_mode());
        }
    }

    pub fn persist(self) {
        let eval = document::eval(
            r#"try { localStorage.setItem('cb-editor', JSON.stringify(await dioxus.recv())); } catch (_) {}"#,
        );
        let _ = eval.send(self);
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum ModalState {
    #[default]
    None,
    NewNote {
        folder: Option<String>,
        title: String,
    },
    NewFolder {
        parent: Option<String>,
        name: String,
    },
    Rename {
        kind: RenameKind,
        value: String,
    },
    Delete {
        kind: DeleteKind,
        title: String,
        message: String,
    },
    /// Project / vault switcher.
    Projects,
    /// App settings (theme, shortcuts, plugins entry).
    Settings,
    /// First-party plugin command palette.
    CommandPalette,
    /// Ask for a string before running a plugin command (Quick Capture / titles).
    PromptValue {
        title: String,
        default: String,
        pending_command: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenameKind {
    Note { slug: String },
    Folder { path: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeleteKind {
    Note { slug: String },
    Folder { path: String },
    File { path: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuState {
    pub x: f64,
    pub y: f64,
    pub items: Vec<ContextMenuItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuItem {
    pub label: String,
    pub danger: bool,
    pub action: ContextAction,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContextAction {
    NewNote {
        folder: Option<String>,
    },
    NewFolder {
        parent: Option<String>,
    },
    RenameNote {
        slug: String,
    },
    RenameFolder {
        path: String,
    },
    DeleteNote {
        slug: String,
    },
    DeleteFolder {
        path: String,
    },
    DeleteFile {
        path: String,
    },
    /// Insert `text` as its own line at the editor caret.
    InsertInNote {
        text: String,
    },
    Copy {
        text: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragPayload {
    Note { slug: String },
    Folder { path: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DragState {
    pub payload: DragPayload,
    /// Display name shown on the floating drag card.
    pub label: String,
    pub over_folder: Option<Option<String>>,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

impl DragState {
    /// Destination hint under the floating drag card.
    pub fn destination_hint(&self) -> String {
        match &self.over_folder {
            Some(Some(path)) => {
                let name = path.rsplit('/').next().unwrap_or(path);
                format!("Move into '{name}'")
            }
            Some(None) | None => "Move into vault root".into(),
        }
    }

    pub fn is_note_dragging(&self, slug: &str) -> bool {
        matches!(&self.payload, DragPayload::Note { slug: s } if s == slug)
    }

    pub fn is_folder_dragging(&self, path: &str) -> bool {
        matches!(&self.payload, DragPayload::Folder { path: p } if p == path)
    }
}
