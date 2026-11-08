// SPDX-License-Identifier: MIT

//! Pure document-edit operations used by the UI (and unit-tested here).

use crate::block::{Block, BlockKind, join_blocks, parse_blocks};

/// Editor document snapshot for deterministic transforms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocState {
    pub text: String,
    pub focus: usize,
    /// Caret within the focused line body (or reveal text).
    pub caret: Option<usize>,
    /// Source-reveal mode: edit raw markdown for this block index.
    pub reveal: Option<(usize, String)>,
}

impl DocState {
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let n = parse_blocks(&text).len().saturating_sub(1);
        Self {
            text,
            focus: n,
            caret: None,
            reveal: None,
        }
    }

    pub fn blocks(&self) -> Vec<Block> {
        parse_blocks(&self.text)
    }

    fn with_blocks(
        blocks: Vec<Block>,
        focus: usize,
        caret: Option<usize>,
        reveal: Option<(usize, String)>,
    ) -> Self {
        Self {
            text: join_blocks(&blocks),
            focus,
            caret,
            reveal,
        }
    }
}

/// Lists, tasks, quotes, headings: Enter/Backspace unwrap/continue markers.
pub fn is_structural(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Bullet { .. }
            | BlockKind::Ordered { .. }
            | BlockKind::Task { .. }
            | BlockKind::Blockquote { .. }
            | BlockKind::Heading { .. }
    )
}

/// Whole-line embeds/links/HR edited as raw source when focused (never split mid-line).
pub fn is_source_only(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::ImageEmbed { .. }
            | BlockKind::WikiLink { .. }
            | BlockKind::MdLink { .. }
            | BlockKind::HorizontalRule
    )
}

pub fn reveal_prefix(block: &Block) -> String {
    match &block.kind {
        BlockKind::Bullet { indent, marker } => format!("{indent}{marker}"),
        BlockKind::Ordered { indent, marker } => format!("{indent}{marker}"),
        BlockKind::Task { indent, checked } => {
            let mark = if *checked { "x" } else { " " };
            format!("{indent}- [{mark}]")
        }
        BlockKind::Blockquote { depth } => ">".repeat(*depth as usize),
        BlockKind::Heading { level } => "#".repeat(*level as usize),
        BlockKind::ImageEmbed { .. }
        | BlockKind::WikiLink { .. }
        | BlockKind::MdLink { .. }
        | BlockKind::HorizontalRule => block.raw.clone(),
        _ => block.raw.clone(),
    }
}

/// Caret offsets are char counts (matches JS `selectionStart` for BMP text),
/// never UTF-8 byte offsets.
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Byte offset of the `char_idx`-th char (clamped to `s.len()`).
pub fn byte_at(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn write_line_as_paragraph(blocks: &mut [Block], index: usize, raw: &str) {
    if index >= blocks.len() {
        return;
    }
    blocks[index] = Block {
        kind: BlockKind::Paragraph,
        raw: raw.to_string(),
    };
}

/// Infer caret after a controlled `<textarea>` rewrite (browser resets to 0).
pub fn infer_caret(old: &str, new: &str) -> usize {
    if new == old {
        return char_len(new);
    }
    let ob: Vec<char> = old.chars().collect();
    let nb: Vec<char> = new.chars().collect();
    let mut i = 0;
    while i < ob.len() && i < nb.len() && ob[i] == nb[i] {
        i += 1;
    }
    if nb.len() >= ob.len() {
        // Insertion (or replace with longer): caret after the new run.
        let inserted = nb.len() - ob.len();
        (i + inserted).min(nb.len())
    } else {
        // Deletion: caret at the divergence point.
        i.min(nb.len())
    }
}

/// Apply a new visible body to the focused block (typing / delete inside the line).
/// Emptying a structural block reveals its marker (`•` body cleared → `-`).
pub fn apply_body(state: &DocState, new_body: &str) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() {
        return state.clone();
    }
    let prev = blocks[index].clone();
    let old_body = match state.reveal.as_ref() {
        Some((i, t)) if *i == index => t.clone(),
        _ => prev.body().to_string(),
    };

    if new_body.is_empty() && is_structural(&prev.kind) {
        let raw = reveal_prefix(&prev);
        let len = char_len(&raw);
        write_line_as_paragraph(&mut blocks, index, &raw);
        return DocState::with_blocks(blocks, index, Some(len), Some((index, raw)));
    }

    let next = prev.with_body(new_body);
    let became_structural = !is_structural(&prev.kind) && is_structural(&next.kind);
    let body_len = char_len(next.body());
    blocks[index] = next;
    let caret = if became_structural {
        Some(body_len)
    } else {
        Some(infer_caret(&old_body, new_body))
    };
    DocState::with_blocks(blocks, index, caret, None)
}

/// Typing in source-reveal mode.
pub fn apply_reveal(state: &DocState, text: &str) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() {
        return state.clone();
    }
    // Cleared the revealed marker (`-` / `1.`) → delete this line and join to
    // the previous one. First line becomes a plain empty paragraph.
    if text.is_empty() {
        return delete_line_join_prev(blocks, index);
    }
    let old = match state.reveal.as_ref() {
        Some((i, t)) if *i == index => t.clone(),
        _ => String::new(),
    };
    let caret_in_text = infer_caret(&old, text);
    // Commit list/heading once the marker + space is present.
    let parsed = Block::parse_line(text);
    if matches!(
        parsed.kind,
        BlockKind::Bullet { .. }
            | BlockKind::Ordered { .. }
            | BlockKind::Task { .. }
            | BlockKind::Blockquote { .. }
            | BlockKind::Heading { .. }
    ) {
        let body_len = char_len(parsed.body());
        let prefix_len = char_len(text) - body_len;
        let caret = caret_in_text.saturating_sub(prefix_len).min(body_len);
        blocks[index] = parsed;
        return DocState::with_blocks(blocks, index, Some(caret), None);
    }
    write_line_as_paragraph(&mut blocks, index, text);
    DocState::with_blocks(
        blocks,
        index,
        Some(caret_in_text),
        Some((index, text.to_string())),
    )
}

/// Delete one character at the end of reveal text (Backspace with caret at EOL).
pub fn reveal_backspace_char(state: &DocState) -> DocState {
    let Some((ri, ref text)) = state.reveal.clone() else {
        return state.clone();
    };
    if ri != state.focus {
        return state.clone();
    }
    if text.is_empty() {
        return backspace(state, true, false);
    }
    let mut chars: Vec<char> = text.chars().collect();
    chars.pop();
    let next: String = chars.into_iter().collect();
    apply_reveal(state, &next)
}

fn unwrap_or_outdent(blocks: &mut [Block], index: usize) -> (usize, Option<(usize, String)>) {
    let block = blocks[index].clone();
    if block.list_indent_level() > 0
        && let Some(next) = block.with_indent_delta(false)
    {
        blocks[index] = next;
        return (0, None);
    }
    let raw = reveal_prefix(&block);
    let len = char_len(&raw);
    write_line_as_paragraph(blocks, index, &raw);
    (len, Some((index, raw)))
}

/// Remove `index` and put the caret at the end of the previous line.
/// The first line is never removed — only cleared to an empty paragraph.
fn delete_line_join_prev(mut blocks: Vec<Block>, index: usize) -> DocState {
    if index == 0 {
        // Keep the first line; clear content if needed, never drop the block.
        if blocks.is_empty() {
            blocks.push(Block {
                kind: BlockKind::Paragraph,
                raw: String::new(),
            });
        } else if !blocks[0].is_empty_content() || is_structural(&blocks[0].kind) {
            write_line_as_paragraph(&mut blocks, 0, "");
        }
        return DocState::with_blocks(blocks, 0, Some(0), None);
    }
    // Char count matches JS `value.length` for BMP text (setSelectionRange).
    let prev_len = blocks[index - 1].body().chars().count();
    blocks.remove(index);
    if blocks.is_empty() {
        blocks.push(Block {
            kind: BlockKind::Paragraph,
            raw: String::new(),
        });
        return DocState::with_blocks(blocks, 0, Some(0), None);
    }
    DocState::with_blocks(blocks, index - 1, Some(prev_len), None)
}

/// Insert `pasted` at `caret` in the focused line (single- or multi-line).
/// Text of the focused line that [`paste_text`] offsets are measured against.
pub fn paste_line_text(state: &DocState) -> String {
    match state.reveal.as_ref() {
        Some((i, t)) if *i == state.focus => t.clone(),
        _ => state
            .blocks()
            .get(state.focus)
            .map(|b| {
                // Source-only rows edit raw (`![[…]]`), same as EnsureSourceReveal.
                if is_source_only(&b.kind) {
                    b.raw.clone()
                } else {
                    b.body().to_string()
                }
            })
            .unwrap_or_default(),
    }
}

/// Where an own-line insert (image embed) goes for a DOM caret `pos` on a line
/// whose input shows `dom_value`. Never splits embeds/links, and falls back to a
/// line boundary when the input and model disagree.
pub fn own_line_insert_pos(state: &DocState, pos: usize, dom_value: &str) -> usize {
    let line = paste_line_text(state);
    let len = char_len(&line);
    let revealing = state
        .reveal
        .as_ref()
        .is_some_and(|(i, _)| *i == state.focus);
    let splittable = !revealing
        && state.blocks().get(state.focus).is_some_and(|b| {
            matches!(
                b.kind,
                BlockKind::Paragraph
                    | BlockKind::Heading { .. }
                    | BlockKind::Bullet { .. }
                    | BlockKind::Ordered { .. }
                    | BlockKind::Task { .. }
                    | BlockKind::Blockquote { .. }
            )
        });
    if splittable && dom_value == line {
        pos.min(len)
    } else if pos == 0 {
        0
    } else {
        len
    }
}

pub fn paste_text(state: &DocState, pasted: &str, caret: usize) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() || pasted.is_empty() {
        return state.clone();
    }

    let revealing = state.reveal.as_ref().is_some_and(|(i, _)| *i == index);
    let cur = blocks[index].clone();
    let body = paste_line_text(state);
    let caret = caret.min(char_len(&body));
    let at = byte_at(&body, caret);

    if !pasted.contains('\n') {
        let mut s = body;
        s.insert_str(at, pasted);
        let new_caret = caret + char_len(pasted);
        if revealing || is_source_only(&cur.kind) {
            return apply_reveal(
                &DocState {
                    caret: Some(caret),
                    reveal: Some((index, paste_line_text(state))),
                    ..state.clone()
                },
                &s,
            );
        }
        if is_structural(&cur.kind) {
            blocks[index] = cur.with_body(&s);
        } else {
            write_line_as_paragraph(&mut blocks, index, &s);
        }
        return DocState::with_blocks(blocks, index, Some(new_caret), None);
    }

    let lines: Vec<&str> = pasted.split('\n').collect();
    let before = &body[..at];
    let after = &body[at..];
    let first = format!("{before}{}", lines[0]);
    // Source-only / reveal: keep first line as raw markdown (re-parse).
    if revealing || is_source_only(&cur.kind) || !is_structural(&cur.kind) {
        write_line_as_paragraph(&mut blocks, index, &first);
        // Re-parse so `![[a.png]]` stays an embed after a split paste.
        if let Some(b) = blocks.get_mut(index) {
            *b = Block::parse_line(&first);
        }
    } else {
        blocks[index] = cur.with_body(&first);
    }

    let last_idx = lines.len() - 1;
    let mut offset = 0usize;
    for (li, line) in lines.iter().enumerate().skip(1) {
        let raw = if li == last_idx {
            format!("{line}{after}")
        } else {
            (*line).to_string()
        };
        offset += 1;
        blocks.insert(index + offset, Block::parse_line(&raw));
    }
    let focus = index + offset;
    let caret_at = blocks.get(focus).map(|b| char_len(b.body())).unwrap_or(0);
    DocState::with_blocks(blocks, focus, Some(caret_at), None)
}

/// Backspace on the focused line.
///
/// * `at_start` — caret at offset 0 (and no selection).
/// * `was_structural` — kind before this keypress (guards the race where emptying
///   a list body already revealed `-` via [`apply_body`]).
pub fn backspace(state: &DocState, at_start: bool, was_structural: bool) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() {
        return state.clone();
    }

    // Reveal mode.
    if let Some((ri, ref text)) = state.reveal.clone() {
        if ri != index {
            return state.clone();
        }
        if !text.is_empty() {
            // Deleting inside reveal text is handled by the input; no line drop.
            return state.clone();
        }
        // Already-empty reveal: remove the line and move up.
        return delete_line_join_prev(blocks, index);
    }

    let b = blocks[index].clone();
    let empty = b.is_empty_content();
    let structural = is_structural(&b.kind);

    if empty {
        if structural {
            let (caret, reveal) = unwrap_or_outdent(&mut blocks, index);
            return DocState::with_blocks(blocks, index, Some(caret), reveal);
        }
        // Empty paragraph / blank → delete line, caret on previous.
        return delete_line_join_prev(blocks, index);
    }

    if !at_start {
        return state.clone();
    }

    // Race guard: body already turned into revealed paragraph by apply_body.
    if was_structural && !structural {
        return state.clone();
    }

    // Backspace at start of a non-empty list/heading deletes the space after
    // the marker and reveals it as source (`• text` → `-text`, caret after `-`).
    if matches!(
        b.kind,
        BlockKind::Bullet { .. }
            | BlockKind::Ordered { .. }
            | BlockKind::Task { .. }
            | BlockKind::Blockquote { .. }
            | BlockKind::Heading { .. }
    ) {
        let prefix = reveal_prefix(&b);
        let raw = format!("{prefix}{}", b.body());
        write_line_as_paragraph(&mut blocks, index, &raw);
        return DocState::with_blocks(blocks, index, Some(char_len(&prefix)), Some((index, raw)));
    }
    if structural {
        let body = b.body().to_string();
        write_line_as_paragraph(&mut blocks, index, &body);
        return DocState::with_blocks(blocks, index, Some(0), None);
    }

    if index == 0 {
        return state.clone();
    }

    // Merge into previous.
    let cur = blocks[index].clone();
    let prev = blocks[index - 1].clone();
    let cur_text = match cur.kind {
        BlockKind::Blank => String::new(),
        BlockKind::Paragraph => cur.raw.clone(),
        _ => cur.body().to_string(),
    };
    let (merged, join_at) = match &prev.kind {
        BlockKind::Blank => (Block::parse_line(&cur_text), 0usize),
        BlockKind::Paragraph => {
            let join_at = char_len(&prev.raw);
            (
                Block::parse_line(&format!("{}{}", prev.raw, cur_text)),
                join_at,
            )
        }
        BlockKind::Bullet { .. }
        | BlockKind::Ordered { .. }
        | BlockKind::Task { .. }
        | BlockKind::Blockquote { .. }
        | BlockKind::Heading { .. } => {
            let join_at = char_len(prev.body());
            (
                prev.with_body(&format!("{}{}", prev.body(), cur_text)),
                join_at,
            )
        }
        _ => {
            let prev_len = char_len(prev.body());
            blocks.remove(index);
            return DocState::with_blocks(blocks, index - 1, Some(prev_len), None);
        }
    };
    blocks[index - 1] = merged;
    blocks.remove(index);
    DocState::with_blocks(blocks, index - 1, Some(join_at), None)
}

/// Enter on the focused line: split at `state.caret` (end of line if `None`);
/// the tail moves to a new line that continues the list marker.
pub fn enter(state: &DocState) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() {
        return state.clone();
    }

    if state.reveal.is_some() {
        blocks.insert(index + 1, Block::parse_line(""));
        return DocState::with_blocks(blocks, index + 1, Some(0), None);
    }

    let cur = &blocks[index];
    if is_structural(&cur.kind) && cur.body().is_empty() {
        // Exit list / empty heading structure → blank line.
        blocks[index] = Block::parse_line("");
        return DocState::with_blocks(blocks, index, Some(0), None);
    }

    let next_prefix = match &cur.kind {
        BlockKind::Bullet { indent, marker } => format!("{indent}{marker} "),
        BlockKind::Task { indent, checked } => {
            let mark = if *checked { "x" } else { " " };
            format!("{indent}- [{mark}] ")
        }
        BlockKind::Blockquote { depth } => "> ".repeat(*depth as usize),
        BlockKind::Ordered { indent, marker } => {
            let n = marker
                .trim_end_matches(['.', ')'])
                .parse::<u32>()
                .unwrap_or(1)
                .saturating_add(1);
            let delim = if marker.ends_with(')') { ')' } else { '.' };
            format!("{indent}{n}{delim} ")
        }
        _ => String::new(),
    };
    // Embeds / links / blank / HR are never split — just open a line below.
    let splittable = matches!(
        cur.kind,
        BlockKind::Paragraph
            | BlockKind::Heading { .. }
            | BlockKind::Bullet { .. }
            | BlockKind::Ordered { .. }
            | BlockKind::Task { .. }
            | BlockKind::Blockquote { .. }
    );
    if !splittable {
        blocks.insert(index + 1, Block::parse_line(&next_prefix));
        return DocState::with_blocks(blocks, index + 1, Some(0), None);
    }
    let body = cur.body().to_string();
    let at = byte_at(&body, state.caret.unwrap_or(usize::MAX));
    let (head, tail) = body.split_at(at);
    let cur_next = if matches!(cur.kind, BlockKind::Paragraph) {
        Block::parse_line(head)
    } else {
        cur.with_body(head)
    };
    let new_line = Block::parse_line(&format!("{next_prefix}{tail}"));
    blocks[index] = cur_next;
    blocks.insert(index + 1, new_line);
    DocState::with_blocks(blocks, index + 1, Some(0), None)
}

/// Tab / Shift+Tab on a list item.
pub fn indent(state: &DocState, deeper: bool) -> DocState {
    let mut blocks = state.blocks();
    let index = state.focus;
    if index >= blocks.len() {
        return state.clone();
    }
    let Some(next) = blocks[index].with_indent_delta(deeper) else {
        return state.clone();
    };
    let caret = state.caret.or(Some(char_len(next.body())));
    blocks[index] = next;
    DocState::with_blocks(blocks, index, caret, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(text: &str, index: usize) -> DocState {
        DocState {
            text: text.into(),
            focus: index,
            caret: None,
            reveal: None,
        }
    }

    #[test]
    fn typing_dash_space_becomes_bullet() {
        let s = focus("", 0);
        let s = apply_body(&s, "-");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Paragraph));
        assert_eq!(s.caret, Some(1), "caret stays after `-`");
        let s = apply_body(&s, "- ");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Bullet { .. }));
        assert_eq!(s.blocks()[0].body(), "");
        assert!(s.reveal.is_none());
    }

    #[test]
    fn infer_caret_insert_and_delete() {
        assert_eq!(infer_caret("", "-"), 1);
        assert_eq!(infer_caret("ab", "axb"), 2);
        assert_eq!(infer_caret("abc", "ac"), 1);
        assert_eq!(infer_caret("hello", "hello!"), 6);
    }

    #[test]
    fn emptying_bullet_body_reveals_dash_not_delete_line() {
        let s = focus("- hello", 0);
        // Delete down to one char…
        let s = apply_body(&s, "h");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Bullet { .. }));
        // …then clear body → reveal `-`, line still exists.
        let s = apply_body(&s, "");
        assert_eq!(s.text, "-");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Paragraph));
        assert_eq!(s.reveal.as_ref().map(|(_, t)| t.as_str()), Some("-"));
        // Race: Backspace at_start after reveal must NOT merge/delete the line.
        let after = backspace(&s, true, true);
        assert_eq!(after.text, "-");
        assert_eq!(after.reveal.as_ref().map(|(_, t)| t.as_str()), Some("-"));
    }

    #[test]
    fn backspace_empty_blank_removes_line() {
        let s = focus("- a\n", 1);
        assert!(matches!(s.blocks()[1].kind, BlockKind::Blank));
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "- a");
        assert_eq!(s.focus, 0);
        assert_eq!(s.caret, Some(1)); // body "a"
    }

    #[test]
    fn backspace_at_start_merges_paragraphs() {
        let s = focus("hello\nworld", 1);
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "helloworld");
        assert_eq!(s.focus, 0);
        assert_eq!(s.caret, Some(5));
    }

    #[test]
    fn own_line_insert_never_splits_embed() {
        let s = focus("![[Pasted image 1.png]]", 0);
        // DOM shows only the name (stale vs raw model) → insert at end of raw.
        let raw_len = char_len("![[Pasted image 1.png]]");
        assert_eq!(own_line_insert_pos(&s, 18, "Pasted image 1.png"), raw_len);
        let at = own_line_insert_pos(&s, 18, "Pasted image 1.png");
        let next = paste_text(&s, "\n![[b.png]]", at);
        assert_eq!(next.text, "![[Pasted image 1.png]]\n![[b.png]]");

        let mut r = focus("![[a.png]]", 0);
        r.reveal = Some((0, "![[a.png]]".into()));
        let at = own_line_insert_pos(&r, 7, "![[a.png]]");
        assert_eq!(at, char_len("![[a.png]]")); // not mid-embed
        let next = paste_text(&r, "\n![[b.png]]", at);
        assert_eq!(next.text, "![[a.png]]\n![[b.png]]");

        let p = focus("hello world", 0);
        assert_eq!(own_line_insert_pos(&p, 5, "hello world"), 5);
        assert_eq!(own_line_insert_pos(&p, 5, "stale"), 11);
    }

    #[test]
    fn backspace_at_start_of_list_deletes_space_and_reveals_marker() {
        let s = focus("- hello", 0);
        let s = backspace(&s, true, true);
        assert_eq!(s.text, "-hello");
        assert_eq!(s.reveal, Some((0, "-hello".into())));
        assert_eq!(s.caret, Some(1));

        let s = focus("1) Go", 0);
        let s = backspace(&s, true, true);
        assert_eq!(s.text, "1)Go");
        assert_eq!(s.caret, Some(2));

        // Next Backspace removes the `-` (native delete in reveal) → plain text.
        let mut r = focus("- hello", 0);
        r = backspace(&r, true, true);
        let r = apply_reveal(&r, "hello");
        assert_eq!(r.text, "hello");
        assert_eq!(r.caret, Some(0));

        // Retyping the space restores the bullet with the caret at body start.
        let mut r2 = focus("- hello", 0);
        r2 = backspace(&r2, true, true);
        let r2 = apply_reveal(&r2, "- hello");
        assert!(matches!(r2.blocks()[0].kind, BlockKind::Bullet { .. }));
        assert_eq!(r2.caret, Some(0));
    }

    #[test]
    fn backspace_empty_bullet_reveals_marker() {
        let s = focus("- ", 0);
        assert!(s.blocks()[0].is_empty_content());
        let s = backspace(&s, true, true);
        assert_eq!(s.text, "-");
        assert_eq!(s.reveal.as_ref().map(|(_, t)| t.as_str()), Some("-"));
        assert_eq!(s.caret, Some(1));
    }

    #[test]
    fn backspace_empty_nested_list_outdents() {
        let s = focus("\t- ", 0);
        assert_eq!(s.blocks()[0].list_indent_level(), 1);
        assert!(s.blocks()[0].is_empty_content());
        let s = backspace(&s, true, true);
        assert!(matches!(s.blocks()[0].kind, BlockKind::Bullet { .. }));
        assert_eq!(s.blocks()[0].list_indent_level(), 0);
        assert!(s.reveal.is_none());
    }

    #[test]
    fn enter_on_empty_list_exits_list() {
        let s = focus("- ", 0);
        assert!(s.blocks()[0].is_empty_content());
        let s = enter(&s);
        assert_eq!(s.text, "");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Blank));
        assert!(s.reveal.is_none());
    }

    #[test]
    fn enter_splits_at_caret() {
        let mut s = focus("Привет мир", 0);
        s.caret = Some(6);
        let out = enter(&s);
        assert_eq!(out.text, "Привет\n мир");
        assert_eq!((out.focus, out.caret), (1, Some(0)));

        let mut s = focus("- один два", 0);
        s.caret = Some(4);
        let out = enter(&s);
        assert_eq!(out.text, "- один\n-  два");

        let mut s = focus("## Title here", 0);
        s.caret = Some(5);
        let out = enter(&s);
        assert_eq!(out.text, "## Title\n here");
    }

    #[test]
    fn enter_continues_bullet_and_ordered() {
        let s = focus("- a", 0);
        let s = enter(&s);
        assert_eq!(s.text, "- a\n- ");
        assert_eq!(s.focus, 1);
        assert!(matches!(s.blocks()[1].kind, BlockKind::Bullet { .. }));

        let s = focus("1) a", 0);
        let s = enter(&s);
        assert_eq!(s.text, "1) a\n2) ");
        assert!(matches!(
            &s.blocks()[1].kind,
            BlockKind::Ordered { marker, .. } if marker == "2)"
        ));

        let s = focus("1. a", 0);
        let s = enter(&s);
        assert_eq!(s.text, "1. a\n2. ");
    }

    #[test]
    fn reveal_dash_space_commits_bullet() {
        let mut s = focus("-", 0);
        s.reveal = Some((0, "-".into()));
        let s = apply_reveal(&s, "- ");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Bullet { .. }));
        assert!(s.reveal.is_none());
    }

    #[test]
    fn reveal_backspace_on_first_line_leaves_empty() {
        let mut s = focus("-", 0);
        s.reveal = Some((0, "-".into()));
        let s = reveal_backspace_char(&s);
        assert_eq!(s.text, "");
        assert!(s.reveal.is_none());
        assert_eq!(s.caret, Some(0));
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn first_line_empty_backspace_is_noop() {
        let s = focus("", 0);
        let out = backspace(&s, true, false);
        assert_eq!(out.text, "");
        assert_eq!(out.focus, 0);
        assert_eq!(out.caret, Some(0));
        // Still one block — first line was not removed.
        assert_eq!(out.blocks().len(), 1);
    }

    #[test]
    fn paste_into_cyrillic_uses_char_offsets() {
        let s = focus("Привет", 0);
        let out = paste_text(&s, "ХХ", 3);
        assert_eq!(out.text, "ПриХХвет");
        assert_eq!(out.caret, Some(5));
        let multi = paste_text(&s, "а\nб", 3);
        assert_eq!(multi.text, "Приа\nбвет");
        assert_eq!(multi.caret, Some(4));
    }

    #[test]
    fn paste_single_and_multiline() {
        let s = focus("ab", 0);
        let s = paste_text(&s, "XY", 1);
        assert_eq!(s.text, "aXYb");
        assert_eq!(s.caret, Some(3));

        let s = focus("ab", 0);
        let s = paste_text(&s, "x\ny", 1);
        assert_eq!(s.text, "ax\nyb");
        assert_eq!(s.focus, 1);
    }

    #[test]
    fn reveal_backspace_moves_to_previous_line() {
        let mut s = focus("- a\n-", 1);
        s.reveal = Some((1, "-".into()));
        let s = reveal_backspace_char(&s);
        assert_eq!(s.text, "- a");
        assert_eq!(s.focus, 0);
        assert_eq!(s.caret, Some(1)); // end of body "a"
        assert!(s.reveal.is_none());
    }

    #[test]
    fn empty_line_backspace_moves_to_previous() {
        let s = focus("hello\n", 1);
        assert!(s.blocks()[1].is_empty_content());
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "hello");
        assert_eq!(s.focus, 0);
        assert_eq!(s.caret, Some(5));
    }

    #[test]
    fn empty_line_backspace_caret_at_end_of_previous() {
        let s = focus("vk x\nandroid\n", 2);
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "vk x\nandroid");
        assert_eq!(s.focus, 1);
        assert_eq!(s.caret, Some("android".chars().count()));
    }

    #[test]
    fn reveal_backspace_char_shortens_marker() {
        let mut s = focus("1.", 0);
        s.reveal = Some((0, "1.".into()));
        let s = reveal_backspace_char(&s);
        assert_eq!(s.text, "1");
        assert_eq!(s.reveal.as_ref().map(|(_, t)| t.as_str()), Some("1"));
        assert_eq!(s.caret, Some(1));
    }

    #[test]
    fn tab_indent_roundtrip() {
        let s = focus("- a", 0);
        let s = indent(&s, true);
        assert_eq!(s.text, "\t- a");
        let s = indent(&s, false);
        assert_eq!(s.text, "- a");
    }

    #[test]
    fn ordered_dot_and_paren_supported() {
        assert!(matches!(
            Block::parse_line("3. item").kind,
            BlockKind::Ordered { .. }
        ));
        assert!(matches!(
            Block::parse_line("3) item").kind,
            BlockKind::Ordered { .. }
        ));
        let s = focus("9. z", 0);
        let s = enter(&s);
        assert!(s.text.ends_with("\n10. "));
    }

    #[test]
    fn backspace_not_at_start_is_noop() {
        let s = focus("hello\nworld", 1);
        let out = backspace(&s, false, false);
        assert_eq!(out, s);
    }

    #[test]
    fn merge_into_list_keeps_marker() {
        let s = focus("- a\nb", 1);
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "- ab");
        assert!(matches!(s.blocks()[0].kind, BlockKind::Bullet { .. }));
        assert_eq!(s.caret, Some(1));
    }

    #[test]
    fn reveal_then_backspace_empty_second_line() {
        // "- a\n-" with focus on revealed second line empty after deleting marker.
        let mut s = focus("- a\n-", 1);
        s.reveal = Some((1, String::new()));
        let s = backspace(&s, true, false);
        assert_eq!(s.text, "- a");
        assert_eq!(s.focus, 0);
    }

    #[test]
    fn apply_body_heading() {
        let s = focus("", 0);
        let s = apply_body(&s, "## ");
        assert!(matches!(
            s.blocks()[0].kind,
            BlockKind::Heading { level: 2 }
        ));
        let s = focus("## ", 0);
        let s = apply_body(&s, "Title");
        assert_eq!(s.blocks()[0].raw, "## Title");
        assert_eq!(s.blocks()[0].body(), "Title");
    }
}
