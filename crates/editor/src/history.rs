// SPDX-License-Identifier: MIT

//! Undo / redo stack for the markdown editor (text + caret/reveal).

use crate::edit::DocState;

const MAX_STEPS: usize = 150;

/// One undo/redo frame: document body plus caret context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSnapshot {
    pub text: String,
    pub focus: usize,
    pub caret: Option<usize>,
    pub reveal: Option<(usize, String)>,
}

impl DocSnapshot {
    pub fn from_doc(doc: &DocState) -> Self {
        Self {
            text: doc.text.clone(),
            focus: doc.focus,
            caret: doc.caret,
            reveal: doc.reveal.clone(),
        }
    }

    pub fn into_doc(self) -> DocState {
        DocState {
            text: self.text,
            focus: self.focus,
            caret: self.caret,
            reveal: self.reveal,
        }
    }
}

impl From<&DocState> for DocSnapshot {
    fn from(doc: &DocState) -> Self {
        Self::from_doc(doc)
    }
}

#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<DocSnapshot>,
    redo: Vec<DocSnapshot>,
    /// When true, consecutive coalesced edits share one undo step.
    coalescing: bool,
}

impl History {
    pub fn record(&mut self, before: &DocState, coalesce: bool) {
        let snap = DocSnapshot::from_doc(before);
        if coalesce && self.coalescing {
            return;
        }
        if self.undo.last().is_some_and(|s| s.text == snap.text) {
            self.coalescing = coalesce;
            return;
        }
        self.undo.push(snap);
        if self.undo.len() > MAX_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.coalescing = coalesce;
    }

    /// Legacy text-only record (tests / callers that only track body).
    pub fn record_text(&mut self, before: &str, coalesce: bool) {
        self.record(
            &DocState {
                text: before.to_string(),
                focus: 0,
                caret: None,
                reveal: None,
            },
            coalesce,
        );
    }

    pub fn break_coalesce(&mut self) {
        self.coalescing = false;
    }

    pub fn undo(&mut self, current: &DocState) -> Option<DocSnapshot> {
        self.coalescing = false;
        let prev = self.undo.pop()?;
        self.redo.push(DocSnapshot::from_doc(current));
        Some(prev)
    }

    pub fn redo(&mut self, current: &DocState) -> Option<DocSnapshot> {
        self.coalescing = false;
        let next = self.redo.pop()?;
        self.undo.push(DocSnapshot::from_doc(current));
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> DocState {
        DocState {
            text: text.into(),
            focus: 0,
            caret: Some(0),
            reveal: None,
        }
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut h = History::default();
        let mut cur = doc("a");
        h.record(&cur, false);
        cur = DocState {
            text: "ab".into(),
            focus: 0,
            caret: Some(2),
            reveal: None,
        };
        h.record(&cur, false);
        cur = DocState {
            text: "abc".into(),
            focus: 0,
            caret: Some(3),
            reveal: None,
        };

        let prev = h.undo(&cur).unwrap();
        assert_eq!(prev.text, "ab");
        assert_eq!(prev.caret, Some(2));
        cur = prev.into_doc();
        let prev = h.undo(&cur).unwrap();
        assert_eq!(prev.text, "a");
        cur = prev.into_doc();
        let next = h.redo(&cur).unwrap();
        assert_eq!(next.text, "ab");
    }

    #[test]
    fn coalesce_keeps_single_undo_step() {
        let mut h = History::default();
        let mut cur = doc("");
        h.record(&cur, true);
        cur = DocState {
            text: "h".into(),
            focus: 0,
            caret: Some(1),
            reveal: None,
        };
        h.record(&cur, true);
        cur = DocState {
            text: "he".into(),
            focus: 0,
            caret: Some(2),
            reveal: None,
        };
        h.record(&cur, true);
        cur = DocState {
            text: "hel".into(),
            focus: 0,
            caret: Some(3),
            reveal: None,
        };
        assert_eq!(h.undo.len(), 1);
        assert_eq!(h.undo[0].text, "");
        let prev = h.undo(&cur).unwrap();
        assert_eq!(prev.text, "");
    }

    #[test]
    fn break_coalesce_starts_new_step() {
        let mut h = History::default();
        h.record(&doc(""), true);
        h.record(
            &DocState {
                text: "a".into(),
                focus: 0,
                caret: Some(1),
                reveal: None,
            },
            true,
        );
        assert_eq!(h.undo.len(), 1);
        assert_eq!(h.undo[0].text, "");
        h.break_coalesce();
        h.record(
            &DocState {
                text: "a".into(),
                focus: 0,
                caret: Some(1),
                reveal: None,
            },
            false,
        );
        assert_eq!(h.undo.len(), 2);
        assert_eq!(h.undo[0].text, "");
        assert_eq!(h.undo[1].text, "a");
        let prev = h
            .undo(&DocState {
                text: "a\nb".into(),
                focus: 1,
                caret: Some(0),
                reveal: None,
            })
            .unwrap();
        assert_eq!(prev.text, "a");
    }

    #[test]
    fn undo_restores_reveal() {
        let mut h = History::default();
        let before = DocState {
            text: "-hello".into(),
            focus: 0,
            caret: Some(1),
            reveal: Some((0, "-hello".into())),
        };
        h.record(&before, false);
        let cur = DocState {
            text: "hello".into(),
            focus: 0,
            caret: Some(0),
            reveal: None,
        };
        let prev = h.undo(&cur).unwrap();
        assert_eq!(prev.reveal, Some((0, "-hello".into())));
        assert_eq!(prev.caret, Some(1));
    }
}
