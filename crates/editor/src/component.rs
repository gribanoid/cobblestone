// SPDX-License-Identifier: MIT

// Event handlers pass the existing editor signals explicitly.
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::block::{Block, BlockKind, code_fence_body_owned, join_blocks, parse_blocks};
use crate::clipboard::{ClipboardContent, PastedImage};
use crate::edit::{self, DocState, apply_body, apply_reveal, is_source_only};
use crate::history::History;
use crate::selection::{self, BlockRange};
use crate::session::{EditorCmd, EditorEffect, EditorSession, cut_doc_sel};

#[derive(Clone, Copy, PartialEq)]
struct EditorText {
    signal: Signal<String>,
    on_change: Option<EventHandler<()>>,
}

impl EditorText {
    fn set(mut self, text: String) {
        if *self.signal.peek() == text {
            return;
        }
        self.signal.set(text);
        if let Some(on_change) = self.on_change {
            on_change.call(());
        }
    }
}

impl std::ops::Deref for EditorText {
    type Target = Signal<String>;

    fn deref(&self) -> &Self::Target {
        &self.signal
    }
}

/// Near-WYSIWYG Markdown editor (line-oriented blocks + fenced code).
#[component]
pub fn MarkdownEditor(
    value: Signal<String>,
    /// Called synchronously after the editor changes the text.
    #[props(default)]
    on_change: Option<EventHandler<()>>,
    #[props(default)] image_srcs: Signal<HashMap<String, String>>,
    #[props(default)] class: String,
    /// Cmd+V with an image on the clipboard; the host saves it and answers
    /// through `insert_embed`.
    #[props(default)]
    on_paste_image: Option<EventHandler<PastedImage>>,
    /// Host → editor: a line (e.g. `![[Pasted image ….png]]`) to insert at the
    /// caret on its own line. Cleared once consumed.
    #[props(default)]
    insert_embed: Option<Signal<Option<String>>>,
) -> Element {
    let value = EditorText {
        signal: value,
        on_change,
    };
    // Open at the start of the file (remounts on note switch via host key).
    let focused = use_signal(|| Some(0usize));
    let focus_gen = use_signal(|| 0u64);
    let caret = use_signal(|| Some(0usize));
    // Pending click-X so onmounted (not a racing spawn) places the caret.
    let mut click_x = use_signal(|| None::<f64>);
    // Raw-markdown edit for a block (`•` → `-`, embed → `![[…]]`, …).
    let mut source_reveal = use_signal(|| None::<(usize, String)>);
    let history = use_signal(History::default);
    // Cross-line selection (native textarea cannot span rows).
    let mut doc_sel = use_signal(|| None::<BlockRange>);
    let mut drag_anchor = use_signal(|| None::<usize>);
    // True when the pointer crossed a row during the current drag.
    let sel_dragged = use_signal(|| false);

    // Capture-phase shortcuts — desktop webview eats Cmd+Z before Dioxus otherwise.
    // Rebind send on every mount: note switches remount the editor and the old
    // dioxus.send channel dies, while a sticky __cbMdShortcutBound flag would
    // leave a dead listener.
    use_hook({
        let mut drag_anchor = drag_anchor;
        move || {
            spawn(async move {
                #[cfg(feature = "desktop")]
                let script = concat!(
                    "const desktopClipboard = true;\n",
                    include_str!("bridge.js")
                );
                #[cfg(not(feature = "desktop"))]
                let script = concat!(
                    "const desktopClipboard = false;\n",
                    include_str!("bridge.js")
                );
                let mut eval = document::eval(script);
                loop {
                    let Ok(op) = eval.recv::<String>().await else {
                        break;
                    };
                    if let Some(text) = op.strip_prefix("copy\n") {
                        // Doc selection: copy from the source of truth, not the JS cache.
                        let text = match *doc_sel.peek() {
                            Some(range) => selection::selected_text(&value.peek(), range),
                            None => text.to_string(),
                        };
                        let _ = crate::clipboard::write_text(&text);
                        continue;
                    }
                    if let Some(text) = op.strip_prefix("paste\n") {
                        handle_paste(
                            text.to_string(),
                            false,
                            value,
                            history,
                            focused,
                            focus_gen,
                            caret,
                            click_x,
                            source_reveal,
                            doc_sel,
                        );
                        continue;
                    }
                    if op == "paste_req" {
                        match crate::clipboard::read_paste() {
                            Some(ClipboardContent::Text(text)) => handle_paste(
                                text,
                                false,
                                value,
                                history,
                                focused,
                                focus_gen,
                                caret,
                                click_x,
                                source_reveal,
                                doc_sel,
                            ),
                            Some(ClipboardContent::Image(img)) => {
                                if let Some(cb) = on_paste_image {
                                    cb.call(img);
                                }
                            }
                            None => {}
                        }
                        continue;
                    }
                    if let Some(rest) = op.strip_prefix("enter|") {
                        let mut parts = rest.splitn(2, '|');
                        let a = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let b = parts.next().and_then(|s| s.parse().ok()).unwrap_or(a);
                        handle_enter(
                            a,
                            b,
                            value,
                            history,
                            focused,
                            caret,
                            click_x,
                            source_reveal,
                            doc_sel,
                        );
                        continue;
                    }
                    if let Some(rest) = op.strip_prefix("cutline|") {
                        let mut parts = rest.splitn(2, '|');
                        let a = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let b = parts.next().and_then(|s| s.parse().ok()).unwrap_or(a);
                        cut_line_range(
                            a,
                            b,
                            value,
                            history,
                            focused,
                            caret,
                            click_x,
                            source_reveal,
                            doc_sel,
                        );
                        continue;
                    }
                    match op.as_str() {
                        "undo" => {
                            dispatch_session(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                doc_sel,
                                click_x,
                                EditorCmd::Undo,
                            );
                        }
                        "redo" => {
                            dispatch_session(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                doc_sel,
                                click_x,
                                EditorCmd::Redo,
                            );
                        }
                        "selectall" => {
                            apply_select_all(
                                value,
                                history,
                                doc_sel,
                                focused,
                                caret,
                                source_reveal,
                                click_x,
                            );
                        }
                        "cut" => {
                            cut_doc_selection(
                                value,
                                history,
                                doc_sel,
                                focused,
                                focus_gen,
                                caret,
                                click_x,
                                source_reveal,
                            );
                        }
                        "delete" => {
                            delete_doc_selection(
                                value,
                                history,
                                doc_sel,
                                focused,
                                focus_gen,
                                caret,
                                click_x,
                                source_reveal,
                            );
                        }
                        "dragend" => {
                            drag_anchor.set(None);
                        }
                        _ => {}
                    }
                }
            });
        }
    });

    use_effect(move || {
        let Some(mut req) = insert_embed else {
            return;
        };
        let Some(line) = req.read().clone() else {
            return;
        };
        req.set(None);
        handle_paste(
            line,
            true,
            value,
            history,
            focused,
            focus_gen,
            caret,
            click_x,
            source_reveal,
            doc_sel,
        );
    });

    let has_sel = doc_sel.read().is_some();
    let host_class = {
        let mut c = if class.is_empty() {
            "cb-md-editor".to_string()
        } else {
            format!("cb-md-editor {class}")
        };
        if has_sel {
            c.push_str(" cb-md-has-sel");
        }
        c
    };

    let blocks = parse_blocks(&value.read());
    let last_idx = blocks.len().saturating_sub(1);
    let focused_idx = *focused.read();
    let sel_range = *doc_sel.read();

    rsx! {
        div {
            class: "{host_class}",
            role: "textbox",
            "aria-multiline": "true",
            tabindex: "0",
            onkeydown: move |e| {
                if try_handle_shortcut(
                    &e,
                    value,
                    history,
                    focused,
                    focus_gen,
                    caret,
                    click_x,
                    source_reveal,
                    doc_sel,
                ) {
                    return;
                }
                // Replace doc selection with the typed character.
                let range = *doc_sel.peek();
                if let Some(range) = range
                    && let Key::Character(ref ch) = e.key()
                        && ch.chars().count() == 1 && !is_modified(&e) {
                            e.prevent_default();
                            let next = selection::replace_range(
                                &value.peek(),
                                range,
                                ch,
                            );
                            doc_sel.set(None);
                            apply_doc(
                                next,
                                value,
                                history,
                                false,
                                focused,
                                focus_gen,
                                caret,
                                click_x,
                                source_reveal,
                                true,
                            true,
                            );
                        }
            },
            onmouseup: move |_| {
                drag_anchor.set(None);
            },
            for (index, block) in blocks.into_iter().enumerate() {
                BlockRow {
                    // Stable key — remounting on focus_gen made lines jump.
                    key: "{index}",
                    index: index,
                    block: block,
                    is_focused: focused_idx == Some(index),
                    is_doc_selected: sel_range.is_some_and(|r| r.contains(index)),
                    image_srcs: image_srcs,
                    value: value,
                    history: history,
                    focused: focused,
                    focus_gen: focus_gen,
                    caret: caret,
                    click_x: click_x,
                    source_reveal: source_reveal,
                    doc_sel: doc_sel,
                    drag_anchor: drag_anchor,
                    sel_dragged: sel_dragged,
                }
            }
            div {
                class: "cb-md-hit-below",
                onclick: move |_| {
                    doc_sel.set(None);
                    source_reveal.set(None);
                    click_x.set(None);
                    focus_block(last_idx, None, focused, focus_gen, caret);
                },
            }
        }
    }
}

fn is_modified(e: &Event<KeyboardData>) -> bool {
    e.modifiers().meta() || e.modifiers().ctrl()
}

fn shortcut_op(e: &Event<KeyboardData>) -> Option<&'static str> {
    if !is_modified(e) {
        return None;
    }
    // Prefer physical key codes — Character+"z" is unreliable with Cmd on some webviews.
    match e.code() {
        Code::KeyZ if e.modifiers().shift() => Some("redo"),
        Code::KeyZ => Some("undo"),
        Code::KeyY => Some("redo"),
        Code::KeyA => Some("selectall"),
        _ => match e.key() {
            Key::Character(ref c) => match c.to_ascii_lowercase().as_str() {
                "z" if e.modifiers().shift() => Some("redo"),
                "z" => Some("undo"),
                "y" => Some("redo"),
                "a" => Some("selectall"),
                _ => None,
            },
            _ => None,
        },
    }
}

/// Cmd/Ctrl shortcuts. Returns true if handled (caller should not run line logic).
fn try_handle_shortcut(
    e: &Event<KeyboardData>,
    value: EditorText,
    history: Signal<History>,
    focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
    doc_sel: Signal<Option<BlockRange>>,
) -> bool {
    match shortcut_op(e) {
        Some("undo") => {
            e.prevent_default();
            e.stop_propagation();
            dispatch_session(
                value,
                history,
                focused,
                caret,
                source_reveal,
                doc_sel,
                click_x,
                EditorCmd::Undo,
            );
            true
        }
        Some("redo") => {
            e.prevent_default();
            e.stop_propagation();
            dispatch_session(
                value,
                history,
                focused,
                caret,
                source_reveal,
                doc_sel,
                click_x,
                EditorCmd::Redo,
            );
            true
        }
        Some("selectall") => {
            e.prevent_default();
            e.stop_propagation();
            apply_select_all(
                value,
                history,
                doc_sel,
                focused,
                caret,
                source_reveal,
                click_x,
            );
            true
        }
        _ => false,
    }
}

fn sync_doc_sel_clipboard(text: &str) {
    let json = json_string_literal(text);
    spawn(async move {
        let _ = document::eval(&format!(r#"window.__cbMdDocSelText = {json};"#));
    });
}

/// Build a session from UI signals, run `cmd`, write state back, apply effects.
fn dispatch_session(
    value: EditorText,
    history: Signal<History>,
    focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    source_reveal: Signal<Option<(usize, String)>>,
    doc_sel: Signal<Option<BlockRange>>,
    click_x: Signal<Option<f64>>,
    cmd: EditorCmd,
) -> EditorEffect {
    let focus = (*focused.peek()).unwrap_or(0);
    let mut session = EditorSession {
        doc: DocState {
            text: value.peek().clone(),
            focus,
            caret: *caret.peek(),
            reveal: source_reveal.peek().clone(),
        },
        history: history.peek().clone(),
        doc_sel: *doc_sel.peek(),
    };
    let fx = session.dispatch(cmd);
    apply_session_to_signals(
        session,
        &fx,
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
    );
    fx
}

fn apply_session_to_signals(
    session: EditorSession,
    fx: &EditorEffect,
    value: EditorText,
    mut history: Signal<History>,
    mut focused: Signal<Option<usize>>,
    mut caret: Signal<Option<usize>>,
    mut source_reveal: Signal<Option<(usize, String)>>,
    mut doc_sel: Signal<Option<BlockRange>>,
    mut click_x: Signal<Option<f64>>,
) {
    history.set(session.history);
    value.set(session.doc.text);
    source_reveal.set(session.doc.reveal);
    caret.set(session.doc.caret);
    doc_sel.set(session.doc_sel);
    click_x.set(None);
    if fx.focus_root {
        focused.set(None);
    } else {
        focused.set(Some(session.doc.focus));
    }
    if let Some(ref t) = fx.clipboard_write {
        let _ = crate::clipboard::write_text(t);
        sync_doc_sel_clipboard(t);
    }
    if fx.clear_native_sel || fx.focus_root {
        clear_native_line_selection();
    }
    if let Some((idx, pos)) = fx.place_caret
        && !fx.focus_root
    {
        restore_line_caret_when(
            Some(idx),
            None,
            Some(pos),
            fx.scroll_into_view,
            fx.expect_line.clone(),
        );
    }
}

fn apply_select_all(
    value: EditorText,
    history: Signal<History>,
    doc_sel: Signal<Option<BlockRange>>,
    focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    source_reveal: Signal<Option<(usize, String)>>,
    click_x: Signal<Option<f64>>,
) {
    dispatch_session(
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
        EditorCmd::SelectAll,
    );
}

fn cut_doc_selection(
    value: EditorText,
    history: Signal<History>,
    doc_sel: Signal<Option<BlockRange>>,
    focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
) {
    let focus = (*focused.peek()).unwrap_or(0);
    let mut session = EditorSession {
        doc: DocState {
            text: value.peek().clone(),
            focus,
            caret: *caret.peek(),
            reveal: source_reveal.peek().clone(),
        },
        history: history.peek().clone(),
        doc_sel: *doc_sel.peek(),
    };
    let fx = cut_doc_sel(&mut session);
    apply_session_to_signals(
        session,
        &fx,
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
    );
}

fn delete_doc_selection(
    value: EditorText,
    history: Signal<History>,
    doc_sel: Signal<Option<BlockRange>>,
    focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
) {
    dispatch_session(
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
        EditorCmd::DeleteSel,
    );
}

/// `own_line`: wrap with newlines as needed so `text` lands on a line of its
/// own (line-level embeds like `![[image.png]]`).
fn handle_paste(
    text: String,
    own_line: bool,
    value: EditorText,
    history: Signal<History>,
    mut focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
    doc_sel: Signal<Option<BlockRange>>,
) {
    // No caret (e.g. a file dropped on the pane): embeds go to the end of the note.
    let had_focus = focused.peek().is_some();
    let last = parse_blocks(&value.peek()).len().saturating_sub(1);
    let index = (*focused.peek()).unwrap_or(if own_line { last } else { 0 });
    focused.set(Some(index));
    spawn(async move {
        let mut eval = document::eval(&format!(
            r#"
            const el = document.querySelector(
              '[data-block-idx="{index}"] .cb-md-input, .cb-md-block-active .cb-md-textarea'
            );
            const v = el ? (el.value || '') : '';
            dioxus.send(el ? [{had_focus} ? (el.selectionStart|0) : v.length, v] : [0, '']);
            "#
        ));
        let (dom_pos, dom_value) = eval
            .recv::<(usize, String)>()
            .await
            .unwrap_or((0, String::new()));
        dispatch_session(
            value,
            history,
            focused,
            caret,
            source_reveal,
            doc_sel,
            click_x,
            EditorCmd::Paste {
                text,
                own_line,
                dom_pos,
                dom_value,
            },
        );
    });
}

/// Enter with the live DOM selection `a..b` (char offsets): drop the selected
/// text, then split the line at `a`.
fn handle_enter(
    a: usize,
    b: usize,
    value: EditorText,
    history: Signal<History>,
    focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
    doc_sel: Signal<Option<BlockRange>>,
) {
    if focused.peek().is_none() {
        return;
    }
    dispatch_session(
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
        EditorCmd::Enter { a, b },
    );
}

fn cut_line_range(
    start: usize,
    end: usize,
    value: EditorText,
    history: Signal<History>,
    focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
    doc_sel: Signal<Option<BlockRange>>,
) {
    if focused.peek().is_none() {
        return;
    }
    dispatch_session(
        value,
        history,
        focused,
        caret,
        source_reveal,
        doc_sel,
        click_x,
        EditorCmd::CutLine { a: start, b: end },
    );
}

fn json_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn clear_native_line_selection() {
    spawn(async move {
        let _ = document::eval(
            r#"(() => {
              const root = document.querySelector('.cb-md-editor');
              if (!root) return;
              // Keep keyboard focus inside the editor (root has tabindex=0);
              // a bare blur() moves focus to <body> and Cmd+C/V/Backspace are lost.
              if (document.activeElement !== root) root.focus({ preventScroll: true });
              const sel = window.getSelection && window.getSelection();
              if (sel && sel.removeAllRanges) sel.removeAllRanges();
              for (const el of root.querySelectorAll('textarea')) {
                try { el.setSelectionRange(0, 0); } catch (_) {}
              }
            })()"#,
        );
    });
}

fn commit_doc(
    value: EditorText,
    mut history: Signal<History>,
    focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    source_reveal: Signal<Option<(usize, String)>>,
    new: String,
    coalesce: bool,
) {
    let prev_text = value.peek().clone();
    if prev_text == new {
        return;
    }
    let before = DocState {
        text: prev_text,
        focus: (*focused.peek()).unwrap_or(0),
        caret: *caret.peek(),
        reveal: source_reveal.peek().clone(),
    };
    history.write().record(&before, coalesce);
    value.set(new);
}

fn focus_block(
    index: usize,
    caret_at: Option<usize>,
    mut focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    mut caret: Signal<Option<usize>>,
) {
    caret.set(caret_at);
    focused.set(Some(index));
    restore_line_caret(Some(index), None, caret_at, true);
}

/// Place caret on a line. Never call `focus()` if already focused — that makes
/// the caret blink/vanish while deleting spaces or typing.
///
/// `hold`: a couple of deferred retries after line-join (Dioxus value= reset).
fn restore_line_caret(
    block_index: Option<usize>,
    click_x: Option<f64>,
    pos: Option<usize>,
    hold: bool,
) {
    restore_line_caret_when(block_index, click_x, pos, hold, None);
}

/// Like [`restore_line_caret`], but waits (frame by frame) until the line's
/// textarea shows `expect` — placing earlier lands on the pre-render value and
/// the caret visibly jumps when Dioxus then rewrites it.
fn restore_line_caret_when(
    block_index: Option<usize>,
    click_x: Option<f64>,
    pos: Option<usize>,
    hold: bool,
    expect: Option<String>,
) {
    spawn(async move {
        let expect_lit = match expect.as_deref() {
            Some(s) => json_string_literal(s),
            None => "null".into(),
        };
        let idx_lit = match block_index {
            Some(i) => i.to_string(),
            None => "null".into(),
        };
        let x_lit = match click_x {
            Some(x) => format!("{x}"),
            None => "null".into(),
        };
        let pos_lit = match pos {
            Some(p) => p.to_string(),
            None => "null".into(),
        };
        let hold_lit = if hold { "true" } else { "false" };
        let _ = document::eval(&format!(
            r#"(() => {{
              const blockIdx = {idx_lit};
              const clickX = {x_lit};
              const wantPos = {pos_lit};
              const hold = {hold_lit};
              const expect = {expect_lit};
              const token = (window.__cbMdCaretToken = (window.__cbMdCaretToken || 0) + 1);

              const resolveEl = () => {{
                if (blockIdx !== null) {{
                  const el = document.querySelector(
                    `[data-block-idx="${{blockIdx}}"] .cb-md-input, [data-block-idx="${{blockIdx}}"] .cb-md-textarea`
                  );
                  if (el) return el;
                }}
                return document.querySelector(
                  '.cb-md-block-active .cb-md-input, .cb-md-block-active .cb-md-textarea'
                );
              }};

              const computePos = (el) => {{
                const val = el.value || '';
                const len = val.length;
                if (clickX !== null && Number.isFinite(clickX)) {{
                  const rect = el.getBoundingClientRect();
                  if (len === 0 || clickX >= rect.right - 4) return len;
                  if (clickX <= rect.left + 2) return 0;
                  const style = getComputedStyle(el);
                  const canvas = document.createElement('canvas');
                  const ctx = canvas.getContext('2d');
                  ctx.font = `${{style.fontWeight}} ${{style.fontSize}} ${{style.fontFamily}}`;
                  let lo = 0, hi = len;
                  while (lo < hi) {{
                    const mid = (lo + hi + 1) >> 1;
                    if (ctx.measureText(val.slice(0, mid)).width <= clickX - rect.left) lo = mid;
                    else hi = mid - 1;
                  }}
                  return lo;
                }}
                if (wantPos === null || wantPos === undefined) return len;
                return Math.min(wantPos, len);
              }};

              const place = () => {{
                if (token !== window.__cbMdCaretToken) return;
                const el = resolveEl();
                if (!el) return false;
                // Critical: do not re-focus an already-focused field.
                if (document.activeElement !== el) {{
                  el.focus({{ preventScroll: true }});
                }}
                const p = computePos(el);
                if (el.selectionStart !== p || el.selectionEnd !== p) {{
                  try {{ el.setSelectionRange(p, p); }} catch (_) {{}}
                }}
                // focus() uses preventScroll — follow the caret manually.
                const sc = el.closest('.cb-md-editor');
                if (sc) {{
                  // Clicks never scroll (the line is already visible under the pointer).
                  const margin = clickX !== null ? 0 : 48;
                  const r = el.getBoundingClientRect();
                  const c = sc.getBoundingClientRect();
                  if (r.bottom > c.bottom - margin) sc.scrollTop += r.bottom - c.bottom + margin;
                  else if (r.top < c.top + margin) sc.scrollTop -= c.top + margin - r.top;
                }}
                return document.activeElement === el;
              }};

              if (expect !== null) {{
                // ~0.5s cap, then place anyway.
                const waitRender = (n) => {{
                  if (token !== window.__cbMdCaretToken) return;
                  const el = resolveEl();
                  if (n < 30 && (!el || el.value !== expect)) {{
                    requestAnimationFrame(() => waitRender(n + 1));
                    return;
                  }}
                  place();
                  if (hold) {{
                    setTimeout(place, 0);
                    setTimeout(place, 48);
                  }}
                }};
                requestAnimationFrame(() => waitRender(0));
                return;
              }}
              requestAnimationFrame(() => {{
                place();
                if (hold) {{
                  setTimeout(place, 0);
                  setTimeout(place, 48);
                }}
              }});
            }})()"#,
        ));
    });
}

/// Ignore blur from line remounts. Do not fight the caret during typing.
fn schedule_blur_cleanup(
    index: usize,
    focused: Signal<Option<usize>>,
    _caret: Signal<Option<usize>>,
    mut source_reveal: Signal<Option<(usize, String)>>,
) {
    spawn(async move {
        let mut eval = document::eval(
            r#"
            await new Promise((r) => setTimeout(r, 40));
            const a = document.activeElement;
            const stay = !!(a && a.closest && a.closest('.cb-md-editor') &&
              (a.classList.contains('cb-md-input') || a.classList.contains('cb-md-textarea') ||
               a.classList.contains('cb-md-editor')));
            dioxus.send(stay);
            "#,
        );
        let stay = eval.recv::<bool>().await.unwrap_or(false);
        if stay {
            return;
        }
        // Another line is focused (we deleted this one) — only clear our reveal
        // and land caret at EOL on the new line once.
        if let Some(new_i) = *focused.peek()
            && new_i != index
        {
            if source_reveal
                .peek()
                .as_ref()
                .is_some_and(|(i, _)| *i == index)
            {
                source_reveal.set(None);
            }
            restore_line_caret(Some(new_i), None, None, true);
            return;
        }
        // Same line still logically focused but DOM blurred — soft refocus once.
        if focused.peek().as_ref() == Some(&index) {
            restore_line_caret(Some(index), None, None, false);
            return;
        }
        if source_reveal
            .peek()
            .as_ref()
            .is_some_and(|(i, _)| *i == index)
        {
            source_reveal.set(None);
        }
    });
}

fn list_indent_style(level: usize) -> String {
    if level == 0 {
        String::new()
    } else {
        format!("padding-left: {}px", level * 24)
    }
}

/// Live textarea selection `(a, b)` on the active line (char offsets).
async fn line_selection_range() -> (usize, usize) {
    let mut eval = document::eval(
        r#"
        const el = document.querySelector(
          '.cb-md-block-active .cb-md-input, .cb-md-block-active .cb-md-textarea'
        );
        if (!el) { dioxus.send([0, 0]); return; }
        dioxus.send([el.selectionStart|0, el.selectionEnd|0]);
        "#,
    );
    eval.recv::<(usize, usize)>().await.unwrap_or((0, 0))
}

fn snapshot_doc(
    value: EditorText,
    _focused: Signal<Option<usize>>,
    caret: Signal<Option<usize>>,
    source_reveal: Signal<Option<(usize, String)>>,
    index: usize,
) -> DocState {
    DocState {
        text: value.peek().clone(),
        focus: index,
        caret: *caret.peek(),
        reveal: source_reveal.peek().clone(),
    }
}

fn apply_doc(
    state: DocState,
    value: EditorText,
    history: Signal<History>,
    coalesce: bool,
    mut focused: Signal<Option<usize>>,
    _focus_gen: Signal<u64>,
    mut caret: Signal<Option<usize>>,
    mut click_x: Signal<Option<f64>>,
    mut source_reveal: Signal<Option<(usize, String)>>,
    place_caret: bool,
    // Strong re-apply after line join / Enter (not for ordinary typing).
    hold_caret: bool,
) {
    let expect = crate::session::expected_line_value(&state);
    commit_doc(
        value,
        history,
        focused,
        caret,
        source_reveal,
        state.text,
        coalesce,
    );
    source_reveal.set(state.reveal);
    click_x.set(None);
    caret.set(state.caret);
    focused.set(Some(state.focus));
    if place_caret {
        restore_line_caret_when(Some(state.focus), None, state.caret, hold_caret, expect);
    }
}

fn enter_reveal_at(
    value: EditorText,
    history: Signal<History>,
    index: usize,
    raw: String,
    caret_at: usize,
    focused: Signal<Option<usize>>,
    focus_gen: Signal<u64>,
    caret: Signal<Option<usize>>,
    click_x: Signal<Option<f64>>,
    source_reveal: Signal<Option<(usize, String)>>,
) {
    let mut blocks = parse_blocks(&value.peek());
    if index >= blocks.len() {
        return;
    }
    blocks[index] = Block {
        kind: BlockKind::Paragraph,
        raw: raw.clone(),
    };
    apply_doc(
        DocState {
            text: join_blocks(&blocks),
            focus: index,
            caret: Some(caret_at),
            reveal: Some((index, raw)),
        },
        value,
        history,
        false,
        focused,
        focus_gen,
        caret,
        click_x,
        source_reveal,
        true,
        true,
    );
}

#[component]
fn BlockRow(
    index: usize,
    block: Block,
    is_focused: bool,
    is_doc_selected: bool,
    image_srcs: Signal<HashMap<String, String>>,
    value: EditorText,
    history: Signal<History>,
    mut focused: Signal<Option<usize>>,
    mut focus_gen: Signal<u64>,
    mut caret: Signal<Option<usize>>,
    mut click_x: Signal<Option<f64>>,
    mut source_reveal: Signal<Option<(usize, String)>>,
    mut doc_sel: Signal<Option<BlockRange>>,
    mut drag_anchor: Signal<Option<usize>>,
    mut sel_dragged: Signal<bool>,
) -> Element {
    let revealing = source_reveal
        .read()
        .as_ref()
        .is_some_and(|(i, _)| *i == index);
    let reveal_text = source_reveal
        .read()
        .as_ref()
        .filter(|(i, _)| *i == index)
        .map(|(_, t)| t.clone());

    let kind_class = if revealing {
        "cb-md-paragraph"
    } else {
        match &block.kind {
            BlockKind::Blank => "cb-md-blank",
            BlockKind::Paragraph => "cb-md-paragraph",
            BlockKind::Heading { level } => match level {
                1 => "cb-md-heading cb-md-h1",
                2 => "cb-md-heading cb-md-h2",
                3 => "cb-md-heading cb-md-h3",
                4 => "cb-md-heading cb-md-h4",
                5 => "cb-md-heading cb-md-h5",
                _ => "cb-md-heading cb-md-h6",
            },
            BlockKind::Bullet { .. } => "cb-md-list-ul",
            BlockKind::Ordered { .. } => "cb-md-list-ol",
            BlockKind::Task { .. } => "cb-md-task",
            BlockKind::Blockquote { .. } => "cb-md-quote",
            BlockKind::HorizontalRule => "cb-md-hr",
            BlockKind::ImageEmbed { .. } => "cb-md-image",
            BlockKind::CodeFence { .. } => "cb-md-code",
            BlockKind::WikiLink { .. } => "cb-md-wikilink",
            BlockKind::MdLink { .. } => "cb-md-mdlink",
        }
    };

    let active_class = {
        let mut c = if is_focused {
            "cb-md-block cb-md-block-active".to_string()
        } else {
            "cb-md-block".to_string()
        };
        if is_doc_selected {
            c.push_str(" cb-md-block-sel");
        }
        c
    };

    let enter_reveal = move |raw: String, caret_at: usize| {
        enter_reveal_at(
            value,
            history,
            index,
            raw,
            caret_at,
            focused,
            focus_gen,
            caret,
            click_x,
            source_reveal,
        );
    };

    let row_mousedown = move |evt: Event<MouseData>| {
        if evt.data().trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
            drag_anchor.set(Some(index));
            sel_dragged.set(false);
        }
    };
    let row_mouseenter = move |_: Event<MouseData>| {
        let Some(anchor) = *drag_anchor.peek() else {
            return;
        };
        if anchor == index {
            return;
        }
        // Crossed into another row → vault-style block range.
        sel_dragged.set(true);
        let range = BlockRange::new(anchor, index);
        doc_sel.set(Some(range));
        // No active line during a block selection (caret hidden, anchor row
        // renders as a hugging span like the others).
        focused.set(None);
        sync_doc_sel_clipboard(&selection::selected_text(&value.peek(), range));
        clear_native_line_selection();
    };
    // End of a cross-line drag keeps the painted selection instead of clicking.
    let drag_click_consumed = move || {
        let (mut dragged, mut sel) = (sel_dragged, doc_sel);
        if *dragged.peek() {
            dragged.set(false);
            return true;
        }
        sel.set(None);
        false
    };

    // Inactive horizontal rule.
    if matches!(block.kind, BlockKind::HorizontalRule) && !is_focused && !revealing {
        let raw = block.raw.clone();
        return rsx! {
            div {
                class: "{active_class} {kind_class}",
                "data-block-idx": "{index}",
                onmousedown: row_mousedown,
                onmouseenter: row_mouseenter,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if drag_click_consumed() {
                        return;
                    }
                    click_x.set(Some(evt.data().client_coordinates().x));
                    enter_reveal(raw.clone(), edit::char_len(&raw));
                },
                hr { class: "cb-md-hr-line" }
            }
        };
    }

    // Inactive image embed.
    if let BlockKind::ImageEmbed { name, width } = &block.kind
        && !is_focused
        && !revealing
    {
        let src = image_srcs.read().get(name).cloned();
        let alt = name.clone();
        let wattr = width.map(|w| w.to_string());
        let raw = block.raw.clone();
        return rsx! {
            div {
                class: "{active_class} {kind_class}",
                "data-block-idx": "{index}",
                onmousedown: row_mousedown,
                onmouseenter: row_mouseenter,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if drag_click_consumed() {
                        return;
                    }
                    click_x.set(Some(evt.data().client_coordinates().x));
                    enter_reveal(raw.clone(), edit::char_len(&raw));
                },
                if let Some(src) = src {
                    img {
                        class: "cb-embed-image",
                        src: "{src}",
                        alt: "{alt}",
                        width: wattr.clone(),
                    }
                } else {
                    div { class: "cb-md-image-missing",
                        "![[{alt}]]"
                    }
                }
            }
        };
    }

    // Inactive wiki / markdown link — show label; click reveals raw syntax.
    if matches!(
        block.kind,
        BlockKind::WikiLink { .. } | BlockKind::MdLink { .. }
    ) && !is_focused
        && !revealing
    {
        let label = block.body().to_string();
        let raw = block.raw.clone();
        let url_hint = match &block.kind {
            BlockKind::MdLink { url, .. } => Some(url.clone()),
            _ => None,
        };
        return rsx! {
            div {
                class: "{active_class} {kind_class}",
                "data-block-idx": "{index}",
                onmousedown: row_mousedown,
                onmouseenter: row_mouseenter,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if drag_click_consumed() {
                        return;
                    }
                    click_x.set(Some(evt.data().client_coordinates().x));
                    enter_reveal(raw.clone(), edit::char_len(&raw));
                },
                span {
                    class: "cb-md-link-label",
                    title: url_hint.clone().unwrap_or_default(),
                    "{label}"
                }
            }
        };
    }

    // Inactive code fence.
    if let BlockKind::CodeFence { lang } = &block.kind
        && !is_focused
        && !revealing
    {
        let body = code_fence_body_owned(&block.raw);
        let lang_label = lang.clone();
        return rsx! {
            div {
                class: "{active_class} {kind_class}",
                "data-block-idx": "{index}",
                onmousedown: row_mousedown,
                onmouseenter: row_mouseenter,
                onclick: move |evt| {
                    evt.stop_propagation();
                    if drag_click_consumed() {
                        return;
                    }
                    click_x.set(Some(evt.data().client_coordinates().x));
                    source_reveal.set(None);
                    focus_block(index, None, focused, focus_gen, caret);
                },
                if !lang_label.is_empty() {
                    div { class: "cb-md-code-lang", "{lang_label}" }
                }
                pre { class: "cb-md-code-pre",
                    code { "{body}" }
                }
            }
        };
    }

    // Focused code fence.
    if is_focused && block.is_code_fence() && !revealing {
        let body = block.body().to_string();
        let caret_pos: Option<usize> = (*caret.peek()).map(|p| p.min(edit::char_len(&body)));
        let lang_label = match &block.kind {
            BlockKind::CodeFence { lang } => lang.clone(),
            _ => String::new(),
        };
        return rsx! {
            div {
                class: "{active_class} {kind_class}",
                onclick: move |evt| evt.stop_propagation(),
                if !lang_label.is_empty() {
                    div { class: "cb-md-code-lang", "{lang_label}" }
                }
                textarea {
                    class: "cb-md-textarea",
                    value: "{body}",
                    spellcheck: false,
                    autofocus: true,
                    rows: "8",
                    onmounted: move |_| {
                        let pending_x = click_x.write().take();
                        restore_line_caret(Some(index), pending_x, caret_pos, true);
                    },
                    oninput: move |e| {
                        let mut blocks = parse_blocks(&value.peek());
                        if index < blocks.len() {
                            blocks[index] = blocks[index].with_body(&e.value());
                            commit_doc(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                join_blocks(&blocks),
                                true,
                            );
                        }
                    },
                    onfocus: move |_| focused.set(Some(index)),
                    onblur: move |_| {
                        history.write().break_coalesce();
                        schedule_blur_cleanup(index, focused, caret, source_reveal);
                    },
                }
            }
        };
    }

    // Embeds / links have no editable "body": sync source-reveal on focus
    // (no async spawn — matches EditorCmd::EnsureSourceReveal).
    let source_only = is_source_only(&block.kind);
    let auto_reveal = is_focused && !revealing && source_only;
    if auto_reveal {
        let raw = block.raw.clone();
        if source_reveal
            .peek()
            .as_ref()
            .is_none_or(|(i, _)| *i != index)
        {
            source_reveal.set(Some((index, raw)));
        }
    }
    // Image under its source line while the caret is on it.
    let embed_preview = match &block.kind {
        BlockKind::ImageEmbed { name, width } if is_focused || revealing => Some((
            image_srcs.read().get(name).cloned(),
            name.clone(),
            width.map(|w| w.to_string()),
        )),
        _ => None,
    };

    // Unified line row: same DOM whether focused or idle (avoids span↔textarea shake).
    let line_value = if let Some(ref t) = reveal_text {
        t.clone()
    } else if auto_reveal {
        block.raw.clone()
    } else {
        block.body().to_string()
    };
    let caret_pos: Option<usize> = (*caret.peek()).map(|p| p.min(edit::char_len(&line_value)));
    let ol_marker = if revealing {
        None
    } else {
        match &block.kind {
            BlockKind::Ordered { marker, .. } => Some(marker.clone()),
            _ => None,
        }
    };
    let show_bullet = !revealing && matches!(block.kind, BlockKind::Bullet { .. });
    let task_checked = match &block.kind {
        BlockKind::Task { checked, .. } if !revealing => Some(*checked),
        _ => None,
    };
    let structural = edit::is_structural(&block.kind);
    let indent_pad = if revealing {
        String::new()
    } else {
        list_indent_style(block.list_indent_level())
    };
    let editing = is_focused || revealing;
    let row_class = if embed_preview.is_some() {
        format!("{active_class} {kind_class} cb-md-embed-src")
    } else {
        format!("{active_class} {kind_class}")
    };

    rsx! {
        div {
            class: "{row_class}",
            style: "{indent_pad}",
            "data-block-idx": "{index}",
            onmousedown: row_mousedown,
            onmouseenter: row_mouseenter,
            onclick: move |evt| {
                evt.stop_propagation();
                if drag_click_consumed() {
                    return;
                }
                let x = evt.data().client_coordinates().x;
                if !is_focused {
                    click_x.set(Some(x));
                    source_reveal.set(None);
                    caret.set(None);
                    focused.set(Some(index));
                }
                restore_line_caret(Some(index), Some(x), None, true);
            },
            if show_bullet {
                span {
                    class: "cb-md-bullet-gutter",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        doc_sel.set(None);
                        click_x.set(None);
                        focused.set(Some(index));
                        restore_line_caret(Some(index), None, Some(0), true);
                    },
                    "•"
                }
            }
            if let Some(checked) = task_checked {
                span {
                    class: if checked { "cb-md-task-gutter cb-md-task-done" } else { "cb-md-task-gutter" },
                    onclick: move |evt| {
                        evt.stop_propagation();
                        doc_sel.set(None);
                        let mut blocks = parse_blocks(&value.peek());
                        if let Some(b) = blocks.get_mut(index) {
                            if let BlockKind::Task { indent, checked } = &b.kind {
                                let next = !*checked;
                                let mark = if next { "x" } else { " " };
                                let body = b.body().to_string();
                                let raw = if body.is_empty() {
                                    format!("{indent}- [{mark}] ")
                                } else {
                                    format!("{indent}- [{mark}] {body}")
                                };
                                *b = Block::parse_line(&raw);
                            }
                            commit_doc(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                join_blocks(&blocks),
                                false,
                            );
                        }
                    },
                    if checked { "☑" } else { "☐" }
                }
            }
            if let Some(label) = ol_marker.clone() {
                span {
                    class: "cb-md-ol-gutter",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        doc_sel.set(None);
                        focused.set(Some(index));
                        restore_line_caret(Some(index), None, Some(0), true);
                    },
                    "{label}"
                }
            }
            // Doc selection: a span hugs the text so the highlight fits each line
            // (a textarea is always full-width). Focus sits on the editor root then.
            if is_doc_selected {
                span { class: "cb-md-sel-text", "{line_value}" }
            } else {
            // Grid twin sizes the textarea to its wrapped text (WKWebView lacks
            // `field-sizing: content`, so a rows=1 textarea would clip long lines).
            div {
                class: "cb-md-grow",
                "data-value": "{line_value}",
            // Always the same editable textarea — no readonly/autofocus toggle
            // (those remount/restyle attributes and shake lines on focus change).
            textarea {
                class: "cb-md-input",
                value: "{line_value}",
                spellcheck: true,
                rows: "1",
                onmounted: move |_| {
                    if editing {
                        let pending_x = click_x.write().take();
                        restore_line_caret(Some(index), pending_x, caret_pos, true);
                    }
                },
                oninput: move |e| {
                    doc_sel.set(None);
                    let t = e.value();
                    let snap = snapshot_doc(
                        value, focused, caret, source_reveal, index,
                    );
                    let revealing_now = source_reveal
                        .peek()
                        .as_ref()
                        .is_some_and(|(i, _)| *i == index);
                    let next = if revealing_now {
                        apply_reveal(&snap, &t)
                    } else {
                        apply_body(&snap, &t)
                    };
                    let jumped = next.reveal != snap.reveal || next.focus != snap.focus;
                    // Always re-place once (controlled value= resets selection).
                    // `hold` only when the line identity changes — never while
                    // deleting spaces / typing on the same line.
                    apply_doc(
                        next,
                        value,
                        history,
                        true,
                        focused,
                        focus_gen,
                        caret,
                        click_x,
                        source_reveal,
                        true,
                        jumped,
                    );
                },
                onfocus: move |_| {
                    focused.set(Some(index));
                },
                onblur: move |_| {
                    history.write().break_coalesce();
                    schedule_blur_cleanup(index, focused, caret, source_reveal);
                },
                onkeydown: move |e| {
                    if try_handle_shortcut(
                        &e,
                        value,
                        history,
                        focused,
                        focus_gen,
                        caret,
                        click_x,
                        source_reveal,
                        doc_sel,
                    ) {
                        return;
                    }
                    if doc_sel.peek().is_some() {
                        match e.key() {
                            Key::Backspace | Key::Delete => {
                                e.prevent_default();
                                delete_doc_selection(
                                    value,
                                    history,
                                    doc_sel,
                                    focused,
                                    focus_gen,
                                    caret,
                                    click_x,
                                    source_reveal,
                                );
                                return;
                            }
                            _ => {}
                        }
                    }
                    match e.key() {
                        Key::Enter => {
                            e.prevent_default();
                            // Fallback when capture JS didn't handle Enter (e.g. code path).
                            let pos = (*caret.peek()).unwrap_or(0);
                            dispatch_session(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                doc_sel,
                                click_x,
                                EditorCmd::Enter { a: pos, b: pos },
                            );
                        }
                        Key::Tab => {
                            e.prevent_default();
                            let deeper = !e.modifiers().shift();
                            dispatch_session(
                                value,
                                history,
                                focused,
                                caret,
                                source_reveal,
                                doc_sel,
                                click_x,
                                EditorCmd::Indent { deeper },
                            );
                        }
                        Key::Backspace => {
                            let was_structural = structural;
                            let empty = snapshot_doc(
                                value, focused, caret, source_reveal, index,
                            )
                            .blocks()
                            .get(index)
                            .is_some_and(|b| b.is_empty_content());
                            if revealing || empty || was_structural {
                                e.prevent_default();
                                spawn(async move {
                                    let (a, b) = line_selection_range().await;
                                    let at_start = a == 0 && b == 0;
                                    dispatch_session(
                                        value,
                                        history,
                                        focused,
                                        caret,
                                        source_reveal,
                                        doc_sel,
                                        click_x,
                                        EditorCmd::Backspace {
                                            at_start: at_start || empty,
                                            a,
                                            b,
                                            was_structural,
                                        },
                                    );
                                });
                            }
                        }
                        _ => {}
                    }
                },
            }
            }
            }
            if let Some((src, name, width)) = embed_preview {
                div { class: "cb-md-embed-preview",
                    if let Some(src) = src {
                        img {
                            class: "cb-embed-image",
                            src: "{src}",
                            alt: "{name}",
                            width: width,
                        }
                    } else {
                        div { class: "cb-md-image-missing", "{name}" }
                    }
                }
            }
        }
    }
}
