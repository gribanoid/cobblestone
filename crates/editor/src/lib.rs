// SPDX-License-Identifier: MIT

//! Near-WYSIWYG Markdown editor for Cobblestone.
//!
//! Line-oriented blocks with plain Markdown as the source of truth.
//! Inactive blocks hide markers (`-` → `•`, `#` → sized heading);
//! the focused block shows muted source. Image embeds `![[name]]` render
//! inline when an `image_srcs` map provides a URL.

pub mod block;
mod clipboard;
mod component;
pub mod edit;
mod history;
pub mod inline;
pub mod selection;
pub mod session;

pub use block::{
    Block, BlockKind, code_fence_body_owned, join_blocks, parse_blocks, splice_block_raw,
};
pub use clipboard::PastedImage;
pub use component::MarkdownEditor;
pub use edit::{
    DocState, apply_body, apply_reveal, backspace, enter, indent, infer_caret, is_source_only,
    is_structural, paste_text, reveal_backspace_char,
};
pub use history::{DocSnapshot, History};
pub use session::{EditorCmd, EditorEffect, EditorSession, cut_doc_sel, expected_line_value};
