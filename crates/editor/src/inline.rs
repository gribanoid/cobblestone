// SPDX-License-Identifier: MIT

//! Inline Markdown spans for live-hide (focused line keeps markers; idle can strip).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineSpan {
    Text(String),
    Strong(String),
    Em(String),
    Strike(String),
    Code(String),
}

/// Tokenize a single line of inline Markdown (`**`, `*`, `~~`, `` ` ``).
/// Nesting is not supported; first matching closer wins.
pub fn tokenize_inline(input: &str) -> Vec<InlineSpan> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut plain = String::new();

    let flush_plain = |plain: &mut String, out: &mut Vec<InlineSpan>| {
        if !plain.is_empty() {
            out.push(InlineSpan::Text(std::mem::take(plain)));
        }
    };

    while i < chars.len() {
        // Inline code `…`
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                flush_plain(&mut plain, &mut out);
                let body: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push(InlineSpan::Code(body));
                i += end + 2;
                continue;
            }
        }
        // Strike ~~…~~
        if chars[i] == '~' && chars.get(i + 1) == Some(&'~') {
            if let Some(rel) = find_close(&chars, i + 2, &['~', '~']) {
                flush_plain(&mut plain, &mut out);
                let body: String = chars[i + 2..rel].iter().collect();
                out.push(InlineSpan::Strike(body));
                i = rel + 2;
                continue;
            }
        }
        // Strong **…**
        if chars[i] == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(rel) = find_close(&chars, i + 2, &['*', '*']) {
                flush_plain(&mut plain, &mut out);
                let body: String = chars[i + 2..rel].iter().collect();
                out.push(InlineSpan::Strong(body));
                i = rel + 2;
                continue;
            }
        }
        // Em *…* (not part of **)
        if chars[i] == '*' && chars.get(i + 1) != Some(&'*') {
            if let Some(rel) = chars[i + 1..].iter().position(|&c| c == '*') {
                let end = i + 1 + rel;
                if end + 1 >= chars.len() || chars[end + 1] != '*' {
                    flush_plain(&mut plain, &mut out);
                    let body: String = chars[i + 1..end].iter().collect();
                    out.push(InlineSpan::Em(body));
                    i = end + 1;
                    continue;
                }
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    flush_plain(&mut plain, &mut out);
    out
}

fn find_close(chars: &[char], from: usize, marker: &[char]) -> Option<usize> {
    let mut i = from;
    while i + marker.len() <= chars.len() {
        if chars[i..i + marker.len()] == *marker {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Visible text with markers stripped (idle live-hide preview).
pub fn inline_display_text(input: &str) -> String {
    tokenize_inline(input)
        .into_iter()
        .map(|s| match s {
            InlineSpan::Text(t)
            | InlineSpan::Strong(t)
            | InlineSpan::Em(t)
            | InlineSpan::Strike(t)
            | InlineSpan::Code(t) => t,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_strong_em_code_strike() {
        let spans = tokenize_inline("a **b** c *d* e `f` g ~~h~~");
        assert_eq!(
            spans,
            vec![
                InlineSpan::Text("a ".into()),
                InlineSpan::Strong("b".into()),
                InlineSpan::Text(" c ".into()),
                InlineSpan::Em("d".into()),
                InlineSpan::Text(" e ".into()),
                InlineSpan::Code("f".into()),
                InlineSpan::Text(" g ".into()),
                InlineSpan::Strike("h".into()),
            ]
        );
    }

    #[test]
    fn display_strips_markers() {
        assert_eq!(inline_display_text("**hi** there"), "hi there");
        assert_eq!(inline_display_text("`code`"), "code");
    }

    #[test]
    fn unmatched_markers_stay_text() {
        let spans = tokenize_inline("a * b");
        assert_eq!(spans, vec![InlineSpan::Text("a * b".into())]);
    }
}
