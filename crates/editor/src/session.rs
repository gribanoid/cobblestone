// SPDX-License-Identifier: MIT

//! Pure editor orchestration (no Dioxus / DOM). UI reads caret from the DOM,
//! builds an [`EditorCmd`], and applies the returned [`EditorEffect`].

use crate::block::{join_blocks, parse_blocks, Block, BlockKind};
use crate::edit::{
    apply_body, apply_reveal, backspace, byte_at, char_len, enter, indent, is_source_only,
    own_line_insert_pos, paste_line_text, paste_text, DocState,
};
use crate::history::History;
use crate::selection::{self, BlockRange};

/// Full editor state that scenario tests and the UI share.
#[derive(Clone, Debug)]
pub struct EditorSession {
    pub doc: DocState,
    pub history: History,
    pub doc_sel: Option<BlockRange>,
}

/// Commands the UI (or tests) send into the session.
#[derive(Clone, Debug)]
pub enum EditorCmd {
    /// Replace focused line body / reveal text (typing).
    Input(String),
    /// Enter with DOM selection `a..b` (char offsets). Selection is deleted first.
    Enter { a: usize, b: usize },
    /// Backspace. `at_start` when caret is 0 and selection is empty.
    /// `a..b` is the live selection (used to delete a range inside the line).
    Backspace {
        at_start: bool,
        a: usize,
        b: usize,
        was_structural: bool,
    },
    DeleteSel,
    Paste {
        text: String,
        own_line: bool,
        dom_pos: usize,
        dom_value: String,
    },
    Indent { deeper: bool },
    Undo,
    Redo,
    SelectAll,
    CutLine { a: usize, b: usize },
    Focus(usize),
    /// Promote focused source-only block into reveal mode (sync, no spawn).
    EnsureSourceReveal,
    /// Click a rendered embed/link: reveal its raw source at `caret_at`.
    ClickReveal { index: usize, caret_at: usize },
    /// Set / clear document block selection (cross-line drag).
    SetDocSel(Option<BlockRange>),
    Open { text: String },
}

/// Side-effects the UI must perform after [`EditorSession::dispatch`].
#[derive(Clone, Debug, Default)]
pub struct EditorEffect {
    pub expect_line: Option<String>,
    pub place_caret: Option<(usize, usize)>,
    pub scroll_into_view: bool,
    pub focus_root: bool,
    pub clipboard_write: Option<String>,
    pub clear_native_sel: bool,
}

impl EditorSession {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            doc: DocState {
                text,
                focus: 0,
                caret: Some(0),
                reveal: None,
            },
            history: History::default(),
            doc_sel: None,
        }
    }

    pub fn open(text: impl Into<String>) -> Self {
        let mut s = Self::new(text);
        s.doc.focus = 0;
        s.doc.caret = Some(0);
        s.doc.reveal = None;
        s
    }

    pub fn text(&self) -> &str {
        &self.doc.text
    }

    pub fn dispatch(&mut self, cmd: EditorCmd) -> EditorEffect {
        match cmd {
            EditorCmd::Open { text } => {
                self.doc = DocState {
                    text,
                    focus: 0,
                    caret: Some(0),
                    reveal: None,
                };
                self.doc_sel = None;
                self.history = History::default();
                self.effect_from_doc(true, false)
            }
            EditorCmd::Focus(index) => {
                self.doc_sel = None;
                let n = parse_blocks(&self.doc.text).len();
                if n == 0 {
                    return EditorEffect::default();
                }
                let index = index.min(n - 1);
                self.doc.focus = index;
                self.doc.reveal = None;
                // Sync source-reveal for embeds/links (replaces async spawn).
                self.ensure_source_reveal_inner();
                let caret = self.doc.caret.unwrap_or(0);
                self.doc.caret = Some(caret);
                self.effect_from_doc(true, false)
            }
            EditorCmd::EnsureSourceReveal => {
                self.ensure_source_reveal_inner();
                self.effect_from_doc(true, false)
            }
            EditorCmd::ClickReveal { index, caret_at } => {
                self.doc_sel = None;
                let mut blocks = parse_blocks(&self.doc.text);
                if index >= blocks.len() {
                    return EditorEffect::default();
                }
                let raw = blocks[index].raw.clone();
                blocks[index] = Block {
                    kind: BlockKind::Paragraph,
                    raw: raw.clone(),
                };
                self.commit(
                    DocState {
                        text: join_blocks(&blocks),
                        focus: index,
                        caret: Some(caret_at.min(char_len(&raw))),
                        reveal: Some((index, raw)),
                    },
                    false,
                );
                let mut fx = self.effect_from_doc(true, false);
                fx.scroll_into_view = true;
                fx
            }
            EditorCmd::SetDocSel(range) => {
                self.doc_sel = range;
                self.doc.reveal = None;
                let mut fx = EditorEffect {
                    focus_root: range.is_some(),
                    clear_native_sel: range.is_some(),
                    ..EditorEffect::default()
                };
                if let Some(r) = range {
                    fx.clipboard_write = Some(selection::selected_text(&self.doc.text, r));
                }
                fx
            }
            EditorCmd::SelectAll => {
                let Some(range) = selection::range_all(&self.doc.text) else {
                    return EditorEffect::default();
                };
                self.doc.reveal = None;
                self.doc_sel = Some(range);
                EditorEffect {
                    focus_root: true,
                    clear_native_sel: true,
                    clipboard_write: Some(self.doc.text.clone()),
                    ..EditorEffect::default()
                }
            }
            EditorCmd::DeleteSel => self.delete_sel(false),
            EditorCmd::Input(text) => self.cmd_input(text),
            EditorCmd::Enter { a, b } => self.cmd_enter(a, b),
            EditorCmd::Backspace {
                at_start,
                a,
                b,
                was_structural,
            } => self.cmd_backspace(at_start, a, b, was_structural),
            EditorCmd::Paste {
                text,
                own_line,
                dom_pos,
                dom_value,
            } => self.cmd_paste(text, own_line, dom_pos, dom_value),
            EditorCmd::Indent { deeper } => {
                self.doc_sel = None;
                let next = indent(&self.doc, deeper);
                self.commit(next, false);
                let mut fx = self.effect_from_doc(true, false);
                fx.scroll_into_view = true;
                fx
            }
            EditorCmd::CutLine { a, b } => self.cmd_cut_line(a, b),
            EditorCmd::Undo => self.cmd_undo(),
            EditorCmd::Redo => self.cmd_redo(),
        }
    }

    fn ensure_source_reveal_inner(&mut self) {
        let blocks = parse_blocks(&self.doc.text);
        let Some(b) = blocks.get(self.doc.focus) else {
            return;
        };
        if is_source_only(&b.kind) {
            if self
                .doc
                .reveal
                .as_ref()
                .is_none_or(|(i, _)| *i != self.doc.focus)
            {
                self.doc.reveal = Some((self.doc.focus, b.raw.clone()));
            }
        }
    }

    fn commit(&mut self, next: DocState, coalesce: bool) {
        if next.text == self.doc.text
            && next.focus == self.doc.focus
            && next.caret == self.doc.caret
            && next.reveal == self.doc.reveal
        {
            return;
        }
        if next.text != self.doc.text {
            self.history.record(&self.doc, coalesce);
        }
        self.doc = next;
    }

    fn effect_from_doc(&self, place: bool, hold_scroll: bool) -> EditorEffect {
        let expect = expected_line_value(&self.doc);
        let place_caret = if place {
            Some((self.doc.focus, self.doc.caret.unwrap_or(0)))
        } else {
            None
        };
        EditorEffect {
            expect_line: expect,
            place_caret,
            scroll_into_view: hold_scroll,
            focus_root: false,
            clipboard_write: None,
            clear_native_sel: false,
        }
    }

    fn delete_sel(&mut self, cut: bool) -> EditorEffect {
        let Some(range) = self.doc_sel.take() else {
            return EditorEffect::default();
        };
        let selected = selection::selected_text(&self.doc.text, range);
        let next = selection::delete_range(&self.doc.text, range);
        self.commit(next, false);
        let mut fx = self.effect_from_doc(true, true);
        fx.clear_native_sel = true;
        fx.scroll_into_view = true;
        if cut {
            fx.clipboard_write = Some(selected);
        }
        fx
    }

    fn cmd_input(&mut self, text: String) -> EditorEffect {
        if let Some(range) = self.doc_sel.take() {
            let next = selection::replace_range(&self.doc.text, range, &text);
            self.commit(next, false);
            return self.effect_from_doc(true, true);
        }
        let revealing = self
            .doc
            .reveal
            .as_ref()
            .is_some_and(|(i, _)| *i == self.doc.focus);
        let next = if revealing {
            apply_reveal(&self.doc, &text)
        } else {
            apply_body(&self.doc, &text)
        };
        self.commit(next, true);
        self.effect_from_doc(true, false)
    }

    fn cmd_enter(&mut self, a: usize, b: usize) -> EditorEffect {
        if let Some(range) = self.doc_sel.take() {
            let next = selection::delete_range(&self.doc.text, range);
            self.commit(next, false);
        }
        let mut snap = self.doc.clone();
        if b > a {
            let revealing = snap.reveal.as_ref().is_some_and(|(i, _)| *i == snap.focus);
            let body = paste_line_text(&snap);
            let mut next_body = body[..byte_at(&body, a)].to_string();
            next_body.push_str(&body[byte_at(&body, b)..]);
            snap = if revealing {
                apply_reveal(&snap, &next_body)
            } else {
                apply_body(&snap, &next_body)
            };
        }
        snap.caret = Some(a);
        let next = enter(&snap);
        self.commit(next, false);
        let mut fx = self.effect_from_doc(true, true);
        fx.scroll_into_view = true;
        fx
    }

    fn cmd_backspace(
        &mut self,
        at_start: bool,
        a: usize,
        b: usize,
        was_structural: bool,
    ) -> EditorEffect {
        if self.doc_sel.is_some() {
            return self.delete_sel(false);
        }
        // Selection inside the line: delete the range.
        if b > a {
            let revealing = self
                .doc
                .reveal
                .as_ref()
                .is_some_and(|(i, _)| *i == self.doc.focus);
            let body = paste_line_text(&self.doc);
            let mut next_body = body[..byte_at(&body, a)].to_string();
            next_body.push_str(&body[byte_at(&body, b)..]);
            let mut next = if revealing {
                apply_reveal(&self.doc, &next_body)
            } else {
                apply_body(&self.doc, &next_body)
            };
            next.caret = Some(a);
            self.commit(next, true);
            return self.effect_from_doc(true, false);
        }
        // Delete one char before caret when not at start / not structural unwrap.
        if !at_start && a > 0 {
            let revealing = self
                .doc
                .reveal
                .as_ref()
                .is_some_and(|(i, _)| *i == self.doc.focus);
            let body = paste_line_text(&self.doc);
            let del_at = a.min(char_len(&body));
            if del_at == 0 {
                let next = backspace(&self.doc, true, was_structural);
                self.commit(next, false);
                return self.effect_from_doc(true, true);
            }
            let mut chars: Vec<char> = body.chars().collect();
            chars.remove(del_at - 1);
            let next_body: String = chars.into_iter().collect();
            let mut next = if revealing {
                apply_reveal(&self.doc, &next_body)
            } else {
                apply_body(&self.doc, &next_body)
            };
            next.caret = Some(del_at - 1);
            self.commit(next, true);
            return self.effect_from_doc(true, false);
        }
        let next = backspace(&self.doc, at_start, was_structural);
        self.commit(next, false);
        let mut fx = self.effect_from_doc(true, true);
        fx.scroll_into_view = true;
        fx
    }

    fn cmd_paste(
        &mut self,
        text: String,
        own_line: bool,
        dom_pos: usize,
        dom_value: String,
    ) -> EditorEffect {
        if let Some(range) = self.doc_sel.take() {
            let next = selection::replace_range(&self.doc.text, range, &text);
            self.commit(next, false);
            let mut fx = self.effect_from_doc(true, true);
            fx.scroll_into_view = true;
            return fx;
        }
        let line_len = char_len(&paste_line_text(&self.doc));
        let (pos, len) = if own_line {
            (
                own_line_insert_pos(&self.doc, dom_pos, &dom_value),
                line_len,
            )
        } else {
            (dom_pos.min(line_len), line_len)
        };
        let text = if own_line {
            let mut t = String::new();
            if pos > 0 {
                t.push('\n');
            }
            t.push_str(&text);
            if pos < len {
                t.push('\n');
            }
            t
        } else {
            text
        };
        let next = paste_text(&self.doc, &text, pos);
        self.commit(next, false);
        let mut fx = self.effect_from_doc(true, true);
        fx.scroll_into_view = true;
        fx
    }

    fn cmd_cut_line(&mut self, a: usize, b: usize) -> EditorEffect {
        let body = paste_line_text(&self.doc);
        let n = char_len(&body);
        let start = a.min(n);
        let end = b.min(n).max(start);
        let cut = body[byte_at(&body, start)..byte_at(&body, end)].to_string();
        let mut next_body = String::new();
        next_body.push_str(&body[..byte_at(&body, start)]);
        next_body.push_str(&body[byte_at(&body, end)..]);
        let revealing = self
            .doc
            .reveal
            .as_ref()
            .is_some_and(|(i, _)| *i == self.doc.focus);
        let mut next = if revealing {
            apply_reveal(&self.doc, &next_body)
        } else {
            apply_body(&self.doc, &next_body)
        };
        next.caret = Some(start);
        self.commit(next, false);
        let mut fx = self.effect_from_doc(true, false);
        fx.clipboard_write = Some(cut);
        fx
    }

    fn cmd_undo(&mut self) -> EditorEffect {
        self.doc_sel = None;
        let Some(prev) = self.history.undo(&self.doc) else {
            return EditorEffect::default();
        };
        self.doc = prev.into_doc();
        let mut fx = self.effect_from_doc(true, true);
        fx.scroll_into_view = true;
        fx
    }

    fn cmd_redo(&mut self) -> EditorEffect {
        self.doc_sel = None;
        let Some(next) = self.history.redo(&self.doc) else {
            return EditorEffect::default();
        };
        self.doc = next.into_doc();
        let mut fx = self.effect_from_doc(true, true);
        fx.scroll_into_view = true;
        fx
    }
}

/// What the focused line's textarea will show once `state` renders.
pub fn expected_line_value(state: &DocState) -> Option<String> {
    if let Some((i, t)) = state.reveal.as_ref() {
        if *i == state.focus {
            return Some(t.clone());
        }
    }
    let block = state.blocks().into_iter().nth(state.focus)?;
    Some(match block.kind {
        BlockKind::ImageEmbed { .. } | BlockKind::WikiLink { .. } | BlockKind::MdLink { .. } => {
            block.raw.clone()
        }
        _ => block.body().to_string(),
    })
}

/// Cut selection to clipboard string + delete (for UI Cmd+X with doc_sel).
pub fn cut_doc_sel(session: &mut EditorSession) -> EditorEffect {
    session.delete_sel(true)
}

#[cfg(test)]
mod scenarios {
    use super::*;
    use crate::edit::is_structural;

    fn sess(text: &str) -> EditorSession {
        EditorSession::open(text)
    }

    fn focus(s: &mut EditorSession, i: usize) {
        s.dispatch(EditorCmd::Focus(i));
    }

    fn type_(s: &mut EditorSession, t: &str) {
        s.dispatch(EditorCmd::Input(t.to_string()));
    }

    fn enter_at(s: &mut EditorSession, a: usize, b: usize) {
        s.dispatch(EditorCmd::Enter { a, b });
    }

    fn bs_at_start(s: &mut EditorSession) {
        let structural = s
            .doc
            .blocks()
            .get(s.doc.focus)
            .is_some_and(|b| is_structural(&b.kind));
        s.dispatch(EditorCmd::Backspace {
            at_start: true,
            a: 0,
            b: 0,
            was_structural: structural,
        });
    }

    fn paste_own(s: &mut EditorSession, text: &str, dom_pos: usize, dom_value: &str) {
        s.dispatch(EditorCmd::Paste {
            text: text.to_string(),
            own_line: true,
            dom_pos,
            dom_value: dom_value.to_string(),
        });
    }

    #[test]
    fn open_file_caret_at_start() {
        let s = sess("hello\nworld");
        assert_eq!(s.doc.focus, 0);
        assert_eq!(s.doc.caret, Some(0));
        assert!(s.doc.reveal.is_none());
    }

    #[test]
    fn backspace_at_start_of_list_reveals_marker() {
        let mut s = sess("- hello");
        focus(&mut s, 0);
        // Focus on bullet shows body "hello"; caret at 0.
        s.doc.caret = Some(0);
        s.doc.reveal = None;
        bs_at_start(&mut s);
        assert_eq!(s.text(), "-hello");
        assert_eq!(s.doc.reveal, Some((0, "-hello".into())));
        assert_eq!(s.doc.caret, Some(1));
    }

    #[test]
    fn enter_splits_at_caret_cyrillic() {
        let mut s = sess("привет мир");
        focus(&mut s, 0);
        s.doc.caret = Some(7); // after "привет "
        enter_at(&mut s, 7, 7);
        assert_eq!(s.text(), "привет \nмир");
        assert_eq!(s.doc.focus, 1);
        assert_eq!(s.doc.caret, Some(0));
    }

    #[test]
    fn paste_own_line_never_splits_embed() {
        let mut s = sess("![[a.png]]");
        focus(&mut s, 0);
        // Ensure reveal raw so paste_line_text is the full embed.
        s.dispatch(EditorCmd::EnsureSourceReveal);
        paste_own(&mut s, "![[b.png]]", 18, "a.png");
        assert_eq!(s.text(), "![[a.png]]\n![[b.png]]");
    }

    #[test]
    fn delete_doc_sel_removes_embed_line() {
        let mut s = sess("a\n![[x.png]]\nb");
        s.dispatch(EditorCmd::SetDocSel(Some(BlockRange::new(1, 1))));
        s.dispatch(EditorCmd::DeleteSel);
        assert_eq!(s.text(), "a\nb");
        assert_eq!(s.doc.focus, 1);
        assert!(s.doc_sel.is_none());
    }

    #[test]
    fn type_dash_space_becomes_bullet_then_empty_enter_exits() {
        let mut s = sess("");
        focus(&mut s, 0);
        type_(&mut s, "- ");
        assert!(matches!(
            s.doc.blocks()[0].kind,
            BlockKind::Bullet { .. }
        ));
        // Empty bullet body + Enter exits list.
        s.doc.caret = Some(0);
        enter_at(&mut s, 0, 0);
        assert!(matches!(
            s.doc.blocks()[0].kind,
            BlockKind::Blank | BlockKind::Paragraph
        ));
        assert_eq!(s.text(), "");
    }

    #[test]
    fn undo_restores_focus_and_caret() {
        let mut s = sess("ab");
        focus(&mut s, 0);
        s.doc.caret = Some(2);
        type_(&mut s, "abc");
        assert_eq!(s.text(), "abc");
        s.dispatch(EditorCmd::Undo);
        assert_eq!(s.text(), "ab");
        // Snapshot from before Input should restore caret context from history.
        assert_eq!(s.doc.focus, 0);
    }

    #[test]
    fn focus_on_embed_ensures_source_reveal() {
        let mut s = sess("![[pic.png]]");
        s.dispatch(EditorCmd::Focus(0));
        assert_eq!(
            s.doc.reveal,
            Some((0, "![[pic.png]]".into()))
        );
        assert!(s.doc_sel.is_none());
    }

    #[test]
    fn focus_does_not_set_doc_sel() {
        let mut s = sess("a\nb");
        s.dispatch(EditorCmd::Focus(1));
        assert!(s.doc_sel.is_none());
    }

    #[test]
    fn select_all_then_delete() {
        let mut s = sess("- a\n- b");
        s.dispatch(EditorCmd::SelectAll);
        assert_eq!(s.doc_sel, Some(BlockRange { start: 0, end: 1 }));
        s.dispatch(EditorCmd::DeleteSel);
        assert_eq!(s.text(), "");
    }

    #[test]
    fn source_only_not_structural() {
        let b = Block::parse_line("![[x.png]]");
        assert!(is_source_only(&b.kind));
        assert!(!is_structural(&b.kind));
    }

    #[test]
    fn cut_line_writes_clipboard_effect() {
        let mut s = sess("hello");
        focus(&mut s, 0);
        s.doc.caret = Some(5);
        let fx = s.dispatch(EditorCmd::CutLine { a: 0, b: 5 });
        assert_eq!(fx.clipboard_write.as_deref(), Some("hello"));
        assert_eq!(s.text(), "");
    }

    #[test]
    fn task_enter_continues_and_empty_exits() {
        let mut s = sess("- [ ] one");
        focus(&mut s, 0);
        s.doc.caret = Some(3);
        enter_at(&mut s, 3, 3);
        assert!(matches!(
            s.doc.blocks()[1].kind,
            BlockKind::Task { checked: false, .. }
        ));
        assert_eq!(s.doc.blocks()[1].body(), "");
        // Empty task + Enter → blank
        enter_at(&mut s, 0, 0);
        assert!(matches!(
            s.doc.blocks()[1].kind,
            BlockKind::Blank | BlockKind::Paragraph
        ));
    }

    #[test]
    fn quote_and_hr_parse_roundtrip() {
        let mut s = sess("> a\n---\n> b");
        assert!(matches!(
            s.doc.blocks()[0].kind,
            BlockKind::Blockquote { .. }
        ));
        assert!(matches!(
            s.doc.blocks()[1].kind,
            BlockKind::HorizontalRule
        ));
        s.dispatch(EditorCmd::SetDocSel(Some(BlockRange::new(1, 1))));
        s.dispatch(EditorCmd::DeleteSel);
        assert_eq!(s.text(), "> a\n> b");
    }

    #[test]
    fn enter_scroll_effect() {
        let mut s = sess("hi");
        focus(&mut s, 0);
        s.doc.caret = Some(2);
        let fx = s.dispatch(EditorCmd::Enter { a: 2, b: 2 });
        assert!(fx.scroll_into_view);
        assert!(fx.place_caret.is_some());
    }

    #[test]
    fn backspace_deletes_char_before_caret_in_reveal() {
        let mut s = sess("-hello");
        s.doc.reveal = Some((0, "-hello".into()));
        s.doc.focus = 0;
        s.doc.caret = Some(1);
        s.dispatch(EditorCmd::Backspace {
            at_start: false,
            a: 1,
            b: 1,
            was_structural: false,
        });
        assert_eq!(s.text(), "hello");
        assert_eq!(s.doc.caret, Some(0));
    }
}
