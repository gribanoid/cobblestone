// SPDX-License-Identifier: MIT

//! Document-level selection across line blocks (Obsidian-style).
//!
//! Native `<textarea>` selection cannot span rows; we track a block range and
//! paint it in CSS, then serve copy/cut/delete from the Markdown source.

use crate::block::{join_blocks, parse_blocks, Block, BlockKind};
use crate::edit::DocState;

/// Inclusive block-index range (document order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockRange {
    pub start: usize,
    pub end: usize,
}

impl BlockRange {
    pub fn new(a: usize, b: usize) -> Self {
        Self {
            start: a.min(b),
            end: a.max(b),
        }
    }

    pub fn contains(self, index: usize) -> bool {
        index >= self.start && index <= self.end
    }

    pub fn is_single(self) -> bool {
        self.start == self.end
    }
}

/// Select every block in `text`.
pub fn range_all(text: &str) -> Option<BlockRange> {
    let n = parse_blocks(text).len();
    if n == 0 {
        return None;
    }
    Some(BlockRange {
        start: 0,
        end: n - 1,
    })
}

/// Markdown source for the selected blocks (joined with newlines).
pub fn selected_text(text: &str, range: BlockRange) -> String {
    let blocks = parse_blocks(text);
    if blocks.is_empty() {
        return String::new();
    }
    let end = range.end.min(blocks.len() - 1);
    let start = range.start.min(end);
    join_blocks(&blocks[start..=end])
}

/// Remove selected blocks; leave one empty paragraph at the hole.
pub fn delete_range(text: &str, range: BlockRange) -> DocState {
    let mut blocks = parse_blocks(text);
    if blocks.is_empty() {
        return DocState::from_text(String::new());
    }
    let end = range.end.min(blocks.len() - 1);
    let start = range.start.min(end);
    blocks.drain(start..=end);
    if blocks.is_empty() {
        blocks.push(Block {
            kind: BlockKind::Paragraph,
            raw: String::new(),
        });
    }
    let focus = start.min(blocks.len() - 1);
    DocState {
        text: join_blocks(&blocks),
        focus,
        caret: Some(0),
        reveal: None,
    }
}

/// Replace the selected blocks with a single paragraph containing `replacement`.
pub fn replace_range(text: &str, range: BlockRange, replacement: &str) -> DocState {
    let mut blocks = parse_blocks(text);
    if blocks.is_empty() {
        return DocState {
            text: replacement.to_string(),
            focus: 0,
            caret: Some(crate::edit::char_len(replacement)),
            reveal: None,
        };
    }
    let end = range.end.min(blocks.len() - 1);
    let start = range.start.min(end);
    blocks.drain(start..=end);
    let inserted_blocks = parse_blocks(replacement);
    if inserted_blocks.is_empty() {
        blocks.insert(
            start,
            Block {
                kind: BlockKind::Paragraph,
                raw: String::new(),
            },
        );
        return DocState {
            text: join_blocks(&blocks),
            focus: start,
            caret: Some(0),
            reveal: None,
        };
    }
    let n = inserted_blocks.len();
    let caret = inserted_blocks
        .last()
        .map(|b| crate::edit::char_len(b.body()))
        .unwrap_or(0);
    for (i, b) in inserted_blocks.into_iter().enumerate() {
        blocks.insert(start + i, b);
    }
    DocState {
        text: join_blocks(&blocks),
        focus: start + n - 1,
        caret: Some(caret),
        reveal: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_all_and_selected_text() {
        let t = "- a\n- b\n- c";
        let r = range_all(t).unwrap();
        assert_eq!(r, BlockRange { start: 0, end: 2 });
        assert_eq!(selected_text(t, r), "- a\n- b\n- c");
        assert_eq!(selected_text(t, BlockRange::new(1, 1)), "- b");
    }

    #[test]
    fn delete_middle_leaves_neighbors() {
        let t = "a\nb\nc";
        let next = delete_range(t, BlockRange::new(1, 1));
        assert_eq!(next.text, "a\nc");
        assert_eq!(next.focus, 1);
    }

    #[test]
    fn delete_all_leaves_empty_paragraph() {
        let t = "- a\n- b";
        let next = delete_range(t, range_all(t).unwrap());
        assert_eq!(next.text, "");
        assert_eq!(next.focus, 0);
    }

    #[test]
    fn replace_range_with_char() {
        let t = "a\nb\nc";
        let next = replace_range(t, BlockRange::new(0, 2), "x");
        assert_eq!(next.text, "x");
        assert_eq!(next.caret, Some(1));
    }
}
