// SPDX-License-Identifier: MIT

import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { HighlightStyle, indentUnit, syntaxHighlighting } from "@codemirror/language";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { Compartment, EditorState, Transaction } from "@codemirror/state";
import { EditorView, drawSelection, keymap, lineNumbers } from "@codemirror/view";
import { tags } from "@lezer/highlight";

const theme = EditorView.theme({
  "&": {
    height: "100%",
    color: "var(--text, #ddd)",
    backgroundColor: "var(--bg, #202020)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "ui-monospace, monospace", lineHeight: "1.55" },
  ".cm-content": { padding: "16px 0", caretColor: "var(--text, #ddd)" },
  ".cm-line": { padding: "0 16px" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--text, #ddd)" },
  ".cm-gutters": {
    color: "var(--subtext, #aaa)",
    backgroundColor: "var(--surface, #181818)",
    borderColor: "var(--border, #333)",
  },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "var(--overlay, #444)",
  },
  ".cm-panels": { color: "var(--text, #ddd)", backgroundColor: "var(--surface, #181818)" },
  ".cm-panel .cm-textfield, .cm-panel .cm-button": {
    color: "inherit",
    background: "var(--surface2, #292929)",
  },
});

const highlighting = syntaxHighlighting(HighlightStyle.define([
  { tag: tags.heading, color: "var(--accent, #8bb9df)" },
  { tag: [tags.link, tags.url], color: "var(--accent, #8bb9df)" },
  { tag: tags.monospace, color: "var(--green, #8bbf91)" },
  { tag: [tags.emphasis, tags.strong], color: "var(--accent2, #dbb375)" },
  { tag: [tags.processingInstruction, tags.quote], color: "var(--subtext, #aaa)" },
]));

export function mountEditor(parent, doc, options, onChange) {
  const preferences = new Compartment();
  const configured = ({ line_numbers, line_wrapping, tab_width }) => [
    line_numbers ? lineNumbers() : [],
    line_wrapping ? EditorView.lineWrapping : [],
    EditorState.tabSize.of(tab_width),
    indentUnit.of(" ".repeat(tab_width)),
  ];
  const view = new EditorView({
    parent,
    doc,
    extensions: [
      markdown(),
      history(),
      drawSelection(),
      highlightSelectionMatches(),
      theme,
      highlighting,
      keymap.of([...searchKeymap, ...historyKeymap, ...defaultKeymap, indentWithTab]),
      preferences.of(configured(options)),
      EditorView.contentAttributes.of({ "aria-label": "Markdown source", spellcheck: "true" }),
      EditorView.updateListener.of(update => {
        if (update.docChanged) onChange(update.state.doc.toString());
      }),
    ],
  });

  return {
    configure(options) {
      view.dispatch({ effects: preferences.reconfigure(configured(options)) });
    },
    insertLine(text) {
      const { from, to } = view.state.selection.main;
      const doc = view.state.doc;
      const prefix = from === doc.lineAt(from).from ? "" : "\n";
      const suffix = to === doc.lineAt(to).to && to < doc.length ? "" : "\n";
      const insert = prefix + text + suffix;
      view.dispatch({
        changes: { from, to, insert },
        selection: { anchor: from + insert.length },
        annotations: Transaction.userEvent.of("input"),
        scrollIntoView: true,
      });
      view.focus();
    },
    destroy() {
      view.destroy();
    },
  };
}
