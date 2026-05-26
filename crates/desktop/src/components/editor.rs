// SPDX-License-Identifier: GPL-3.0-or-later
//
// Cobblestone — open-source knowledge base for your private thoughts
// Copyright (C) 2026  Cobblestone Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use leptos::prelude::*;

#[component]
pub fn Editor(
    content:    Signal<String>,
    is_editing: Signal<bool>,
    on_change:  Callback<String>,
) -> impl IntoView {
    view! {
        <div class="editor-wrap">
            // ── Raw editor (visible in edit mode) ──────────────────────────
            <textarea
                class=move || if is_editing.get() { "editor" } else { "editor hidden" }
                spellcheck="true"
                placeholder="Start writing in Markdown…"
                prop:value=content
                on:input=move |ev| on_change.run(event_target_value(&ev))
            />

            // ── Preview (visible in preview mode) ──────────────────────────
            {move || {
                if is_editing.get() {
                    view! { <div class="preview hidden"></div> }.into_any()
                } else {
                    let html = parse_markdown(&content.get());
                    view! {
                        <div class="preview" inner_html=html />
                    }.into_any()
                }
            }}
        </div>
    }
}

// ---------------------------------------------------------------------------
// Minimal Markdown → HTML block parser
// ---------------------------------------------------------------------------

pub fn parse_markdown(md: &str) -> String {
    let mut html  = String::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // ── Fenced code block ─────────────────────────────────────────────
        if line.starts_with("```") {
            let lang = line.trim_start_matches('`').trim();
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code.push_str(&esc_html(lines[i]));
                code.push('\n');
                i += 1;
            }
            html.push_str(&format!(
                "<pre><code class=\"lang-{lang}\">{code}</code></pre>\n"
            ));
            i += 1;
            continue;
        }

        // ── ATX Headings (#, ##, … ######) ───────────────────────────────
        let hashes = line.bytes().take_while(|&b| b == b'#').count();
        if hashes >= 1
            && hashes <= 6
            && line.as_bytes().get(hashes).copied() == Some(b' ')
        {
            let text = esc_html(&line[hashes + 1..]);
            html.push_str(&format!(
                "<h{hashes}>{}</h{hashes}>\n",
                inline_md(&text)
            ));
            i += 1;
            continue;
        }

        // ── Horizontal rule (---, ***, ___) ──────────────────────────────
        {
            let t = line.trim();
            if t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_')) {
                html.push_str("<hr>\n");
                i += 1;
                continue;
            }
        }

        // ── Blockquote ────────────────────────────────────────────────────
        if line.starts_with("> ") || line == ">" {
            let mut inner = String::new();
            while i < lines.len() && (lines[i].starts_with("> ") || lines[i] == ">") {
                inner.push_str(lines[i].strip_prefix("> ").unwrap_or(""));
                inner.push('\n');
                i += 1;
            }
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                parse_markdown(inner.trim())
            ));
            continue;
        }

        // ── Unordered list (- or *) ───────────────────────────────────────
        if line.starts_with("- ") || line.starts_with("* ") {
            html.push_str("<ul>\n");
            while i < lines.len()
                && (lines[i].starts_with("- ") || lines[i].starts_with("* "))
            {
                let t = &lines[i][2..];
                let item_html = if let Some(task) = t.strip_prefix("[ ] ") {
                    format!(
                        "<li class=\"task-item\"><input type=\"checkbox\" disabled> {}</li>\n",
                        inline_md(&esc_html(task))
                    )
                } else if let Some(task) = t.strip_prefix("[x] ").or_else(|| t.strip_prefix("[X] ")) {
                    format!(
                        "<li class=\"task-item done\"><input type=\"checkbox\" checked disabled> {}</li>\n",
                        inline_md(&esc_html(task))
                    )
                } else {
                    format!("<li>{}</li>\n", inline_md(&esc_html(t)))
                };
                html.push_str(&item_html);
                i += 1;
            }
            html.push_str("</ul>\n");
            continue;
        }

        // ── Ordered list (1. 2. …) ────────────────────────────────────────
        if is_ordered_list_item(line) {
            html.push_str("<ol>\n");
            while i < lines.len() && is_ordered_list_item(lines[i]) {
                let rest = lines[i]
                    .trim_start_matches(|c: char| c.is_ascii_digit())
                    .trim_start_matches(". ");
                html.push_str(&format!("<li>{}</li>\n", inline_md(&esc_html(rest))));
                i += 1;
            }
            html.push_str("</ol>\n");
            continue;
        }

        // ── Table (simple GitHub-flavored) ────────────────────────────────
        if line.contains('|')
            && i + 1 < lines.len()
            && is_table_sep(lines[i + 1])
        {
            let headers: Vec<&str> = line.split('|').map(str::trim).filter(|s| !s.is_empty()).collect();
            html.push_str("<table>\n<thead>\n<tr>");
            for h in &headers {
                html.push_str(&format!("<th>{}</th>", inline_md(&esc_html(h))));
            }
            html.push_str("</tr>\n</thead>\n<tbody>\n");
            i += 2; // skip header + separator
            while i < lines.len() && lines[i].contains('|') && !is_table_sep(lines[i]) {
                let cells: Vec<&str> = lines[i].split('|').map(str::trim).filter(|s| !s.is_empty()).collect();
                html.push_str("<tr>");
                for c in &cells {
                    html.push_str(&format!("<td>{}</td>", inline_md(&esc_html(c))));
                }
                html.push_str("</tr>\n");
                i += 1;
            }
            html.push_str("</tbody>\n</table>\n");
            continue;
        }

        // ── Empty line ────────────────────────────────────────────────────
        if line.trim().is_empty() {
            html.push_str("<br>\n");
            i += 1;
            continue;
        }

        // ── Paragraph (greedy: consume until a block-level element) ───────
        let mut para = String::new();
        while i < lines.len() && !is_block_start(lines[i]) {
            para.push_str(lines[i]);
            para.push(' ');
            i += 1;
        }
        let para = para.trim();
        if !para.is_empty() {
            html.push_str(&format!("<p>{}</p>\n", inline_md(&esc_html(para))));
        }
    }

    html
}

// ---------------------------------------------------------------------------
// Inline Markdown → HTML (bold, italic, code, links, images, strikethrough)
//
// The input `s` is assumed to already be HTML-escaped (via `esc_html`).
// We only introduce safe HTML tags here.
// ---------------------------------------------------------------------------

fn inline_md(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(s.len());
    let mut i   = 0;

    while i < n {
        // ── Inline code `…` ──────────────────────────────────────────────
        if chars[i] == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '`') {
                let code: String = chars[i + 1..i + 1 + end].iter().collect();
                out.push_str(&format!("<code>{code}</code>"));
                i += end + 2;
                continue;
            }
        }

        // ── Image ![alt](url) ─────────────────────────────────────────────
        if chars[i] == '!'
            && chars.get(i + 1) == Some(&'[')
        {
            if let Some((alt, url, advance)) = parse_link(&chars, i + 1) {
                out.push_str(&format!(
                    r#"<img src="{}" alt="{}" style="max-width:100%">"#,
                    esc_attr(&url),
                    esc_attr(&alt)
                ));
                i += 1 + advance;
                continue;
            }
        }

        // ── Link [text](url) ──────────────────────────────────────────────
        if chars[i] == '[' {
            if let Some((text, url, advance)) = parse_link(&chars, i) {
                out.push_str(&format!(
                    r#"<a href="{}" target="_blank">{text}</a>"#,
                    esc_attr(&url)
                ));
                i += advance;
                continue;
            }
        }

        // ── Bold + italic ***text*** ──────────────────────────────────────
        if chars.get(i..i + 3) == Some(&['*', '*', '*']) {
            if let Some(end) = find_closing(&chars, i + 3, "***") {
                let inner: String = chars[i + 3..end].iter().collect();
                out.push_str(&format!("<strong><em>{}</em></strong>", inline_md(&inner)));
                i = end + 3;
                continue;
            }
        }

        // ── Bold **text** or __text__ ─────────────────────────────────────
        if chars.get(i..i + 2) == Some(&['*', '*'])
            || chars.get(i..i + 2) == Some(&['_', '_'])
        {
            let delim = if chars[i] == '*' { "**" } else { "__" };
            if let Some(end) = find_closing(&chars, i + 2, delim) {
                let inner: String = chars[i + 2..end].iter().collect();
                out.push_str(&format!("<strong>{}</strong>", inline_md(&inner)));
                i = end + 2;
                continue;
            }
        }

        // ── Italic *text* or _text_ ───────────────────────────────────────
        if (chars[i] == '*' || chars[i] == '_')
            && chars.get(i + 1) != Some(&chars[i])   // not ** or __
        {
            let delim = if chars[i] == '*' { "*" } else { "_" };
            if let Some(end) = find_closing(&chars, i + 1, delim) {
                let inner: String = chars[i + 1..end].iter().collect();
                out.push_str(&format!("<em>{}</em>", inline_md(&inner)));
                i = end + 1;
                continue;
            }
        }

        // ── Strikethrough ~~text~~ ────────────────────────────────────────
        if chars.get(i..i + 2) == Some(&['~', '~']) {
            if let Some(end) = find_closing(&chars, i + 2, "~~") {
                let inner: String = chars[i + 2..end].iter().collect();
                out.push_str(&format!("<del>{}</del>", inline_md(&inner)));
                i = end + 2;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

// ---------------------------------------------------------------------------
// Parser sub-helpers
// ---------------------------------------------------------------------------

/// Try to parse `[text](url)` starting at `chars[start]` (the `[`).
/// Returns `Some((text, url, bytes_consumed))` on success.
fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    if chars.get(start) != Some(&'[') { return None; }
    let close_bracket = find_char(chars, start + 1, ']')?;
    if chars.get(close_bracket + 1) != Some(&'(') { return None; }
    let close_paren = find_char(chars, close_bracket + 2, ')')?;
    let text: String = chars[start + 1..close_bracket].iter().collect();
    let url:  String = chars[close_bracket + 2..close_paren].iter().collect();
    Some((text, url, close_paren + 1 - start))
}

fn find_char(chars: &[char], from: usize, target: char) -> Option<usize> {
    chars[from..].iter().position(|&c| c == target).map(|p| from + p)
}

/// Find the next occurrence of multi-char `delim` in `chars[from..]`.
/// Returns the index of the first character of the delimiter.
fn find_closing(chars: &[char], from: usize, delim: &str) -> Option<usize> {
    let dc: Vec<char> = delim.chars().collect();
    let dl = dc.len();
    if from + dl > chars.len() { return None; }
    chars[from..chars.len() - dl + 1]
        .windows(dl)
        .position(|w| w == dc.as_slice())
        .map(|p| from + p)
}

// ---------------------------------------------------------------------------
// Block-level predicate helpers
// ---------------------------------------------------------------------------

fn is_block_start(line: &str) -> bool {
    if line.trim().is_empty() { return true; }
    if line.starts_with("```") { return true; }
    if line.starts_with("- ") || line.starts_with("* ") { return true; }
    if line.starts_with("> ") || line == ">" { return true; }
    if is_ordered_list_item(line) { return true; }
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if hashes >= 1 && hashes <= 6 && line.as_bytes().get(hashes).copied() == Some(b' ') {
        return true;
    }
    let t = line.trim();
    t.len() >= 3
        && (t.chars().all(|c| c == '-')
            || t.chars().all(|c| c == '*')
            || t.chars().all(|c| c == '_'))
}

fn is_ordered_list_item(line: &str) -> bool {
    let digits: &str = line.trim_start_matches(|c: char| c.is_ascii_digit());
    !digits.is_empty() && line.len() > digits.len() && digits.starts_with(". ")
}

fn is_table_sep(line: &str) -> bool {
    line.trim()
        .chars()
        .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
        && line.contains('-')
}

// ---------------------------------------------------------------------------
// HTML escaping
// ---------------------------------------------------------------------------

fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

fn esc_attr(s: &str) -> String {
    s.replace('"', "&quot;").replace('\'', "&#x27;")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── inline_md ─────────────────────────────────────────────────────────

    #[test]
    fn inline_bold_star() {
        assert_eq!(inline_md("**bold**"), "<strong>bold</strong>");
    }

    #[test]
    fn inline_bold_underscore() {
        assert_eq!(inline_md("__bold__"), "<strong>bold</strong>");
    }

    #[test]
    fn inline_italic_star() {
        assert_eq!(inline_md("*italic*"), "<em>italic</em>");
    }

    #[test]
    fn inline_italic_underscore() {
        assert_eq!(inline_md("_italic_"), "<em>italic</em>");
    }

    #[test]
    fn inline_bold_italic() {
        assert_eq!(inline_md("***both***"), "<strong><em>both</em></strong>");
    }

    #[test]
    fn inline_strikethrough() {
        assert_eq!(inline_md("~~gone~~"), "<del>gone</del>");
    }

    #[test]
    fn inline_code() {
        assert_eq!(inline_md("`code`"), "<code>code</code>");
    }

    #[test]
    fn inline_link() {
        let out = inline_md("[text](https://example.com)");
        assert!(out.contains(r#"href="https://example.com""#));
        assert!(out.contains("text"));
    }

    #[test]
    fn inline_image() {
        let out = inline_md("![alt text](img.png)");
        assert!(out.contains("<img"));
        assert!(out.contains(r#"src="img.png""#));
        assert!(out.contains(r#"alt="alt text""#));
    }

    #[test]
    fn inline_plain_text_unchanged() {
        assert_eq!(inline_md("hello world"), "hello world");
    }

    // ── parse_markdown ────────────────────────────────────────────────────

    #[test]
    fn block_h1() {
        assert!(parse_markdown("# Title").contains("<h1>"));
        assert!(parse_markdown("# Title").contains("</h1>"));
    }

    #[test]
    fn block_h2() {
        assert!(parse_markdown("## Section").contains("<h2>"));
    }

    #[test]
    fn block_hr() {
        assert!(parse_markdown("---").contains("<hr>"));
        assert!(parse_markdown("***").contains("<hr>"));
    }

    #[test]
    fn block_unordered_list() {
        let out = parse_markdown("- item one\n- item two");
        assert!(out.contains("<ul>"));
        assert!(out.contains("<li>item one</li>"));
        assert!(out.contains("<li>item two</li>"));
    }

    #[test]
    fn block_ordered_list() {
        let out = parse_markdown("1. first\n2. second");
        assert!(out.contains("<ol>"));
        assert!(out.contains("<li>first</li>"));
    }

    #[test]
    fn block_task_list_unchecked() {
        let out = parse_markdown("- [ ] task");
        assert!(out.contains("task-item"));
        assert!(!out.contains("checked"));
    }

    #[test]
    fn block_task_list_checked() {
        let out = parse_markdown("- [x] done");
        assert!(out.contains("checked"));
        assert!(out.contains("done"));
    }

    #[test]
    fn block_blockquote() {
        let out = parse_markdown("> quoted text");
        assert!(out.contains("<blockquote>"));
        assert!(out.contains("quoted text"));
    }

    #[test]
    fn block_code_fence() {
        let out = parse_markdown("```rust\nfn main() {}\n```");
        assert!(out.contains("<pre>"));
        assert!(out.contains("<code"));
        assert!(out.contains("fn main()"));
    }

    #[test]
    fn block_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let out = parse_markdown(md);
        assert!(out.contains("<table>"));
        assert!(out.contains("<th>A</th>"));
        assert!(out.contains("<td>1</td>"));
    }

    #[test]
    fn block_paragraph() {
        let out = parse_markdown("Hello world");
        assert!(out.contains("<p>Hello world</p>"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse_markdown(""), "");
    }

    #[test]
    fn xss_in_content_is_escaped() {
        let out = parse_markdown("<script>alert(1)</script>");
        assert!(!out.contains("<script>"));
        assert!(out.contains("&lt;script&gt;"));
    }
}
