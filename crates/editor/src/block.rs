// SPDX-License-Identifier: MIT

//! Line-oriented markdown blocks for the near-WYSIWYG editor.
//! Fenced code blocks (` ```lang ` … ` ``` `) span multiple lines as one block.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Blank,
    Paragraph,
    Heading {
        level: u8,
    },
    Bullet {
        /// Indentation before the marker (spaces/tabs).
        indent: String,
        marker: char,
    },
    Ordered {
        indent: String,
        /// e.g. `"1."` or `"2)"`.
        marker: String,
    },
    /// Task list item: `- [ ] body` / `- [x] body`.
    Task {
        indent: String,
        checked: bool,
    },
    /// Block quote: `> text` (depth = number of `>`).
    Blockquote {
        depth: u8,
    },
    /// Thematic break: `---`, `***`, or `___`.
    HorizontalRule,
    ImageEmbed {
        name: String,
        width: Option<u32>,
    },
    /// Fenced code: ` ```lang\n…\n``` ` (closing fence optional while typing).
    CodeFence {
        lang: String,
    },
    /// Whole-line wikilink `[[Note Title]]`.
    WikiLink {
        title: String,
    },
    /// Whole-line markdown link `[text](url)`.
    MdLink {
        text: String,
        url: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    /// Full source for this block (may contain `\n` for code fences).
    pub raw: String,
}

impl Block {
    pub fn parse_line(line: &str) -> Self {
        let raw = line.to_string();
        if line.is_empty() {
            return Self {
                kind: BlockKind::Blank,
                raw,
            };
        }

        if let Some(embed) = parse_image_embed(line) {
            return Self {
                kind: BlockKind::ImageEmbed {
                    name: embed.0,
                    width: embed.1,
                },
                raw,
            };
        }

        if let Some(title) = parse_wikilink_line(line) {
            return Self {
                kind: BlockKind::WikiLink { title },
                raw,
            };
        }

        if let Some((text, url)) = parse_md_link_line(line) {
            return Self {
                kind: BlockKind::MdLink { text, url },
                raw,
            };
        }

        if is_horizontal_rule(line) {
            return Self {
                kind: BlockKind::HorizontalRule,
                raw,
            };
        }

        if let Some((level, _)) = parse_heading(line) {
            return Self {
                kind: BlockKind::Heading { level },
                raw,
            };
        }

        if let Some((indent, checked)) = parse_task(line) {
            return Self {
                kind: BlockKind::Task { indent, checked },
                raw,
            };
        }

        if let Some((indent, marker, _)) = parse_bullet(line) {
            return Self {
                kind: BlockKind::Bullet { indent, marker },
                raw,
            };
        }

        if let Some((indent, marker, _)) = parse_ordered(line) {
            return Self {
                kind: BlockKind::Ordered { indent, marker },
                raw,
            };
        }

        if let Some(depth) = parse_blockquote(line) {
            return Self {
                kind: BlockKind::Blockquote { depth },
                raw,
            };
        }

        Self {
            kind: BlockKind::Paragraph,
            raw,
        }
    }

    /// Visible body without markdown markers (for inactive rendering).
    pub fn body(&self) -> &str {
        match &self.kind {
            BlockKind::Blank => "",
            BlockKind::Paragraph => self.raw.as_str(),
            BlockKind::HorizontalRule => "",
            BlockKind::Heading { level } => {
                let n = *level as usize;
                let rest = self.raw.get(n..).unwrap_or("");
                rest.strip_prefix(' ').unwrap_or(rest)
            }
            BlockKind::Task { indent, .. } => {
                // `- [ ] ` or `- [x] ` after indent
                let prefix_len = indent.len() + 6;
                self.raw.get(prefix_len..).unwrap_or("")
            }
            BlockKind::Blockquote { depth } => {
                let mut rest = self.raw.as_str();
                for _ in 0..*depth {
                    rest = rest.strip_prefix('>').unwrap_or(rest);
                    rest = rest.strip_prefix(' ').unwrap_or(rest);
                }
                rest
            }
            BlockKind::Bullet { indent, marker: _ } => {
                let prefix_len = indent.len() + 1;
                let rest = self.raw.get(prefix_len..).unwrap_or("");
                rest.strip_prefix(' ').unwrap_or(rest)
            }
            BlockKind::Ordered { indent, marker } => {
                let prefix_len = indent.len() + marker.len();
                let rest = self.raw.get(prefix_len..).unwrap_or("");
                rest.strip_prefix(' ').unwrap_or(rest)
            }
            BlockKind::ImageEmbed { name, .. } => name.as_str(),
            BlockKind::CodeFence { .. } => code_fence_body(&self.raw),
            BlockKind::WikiLink { title } => title.as_str(),
            BlockKind::MdLink { text, .. } => text.as_str(),
        }
    }

    pub fn is_empty_content(&self) -> bool {
        match &self.kind {
            BlockKind::Blank => true,
            BlockKind::HorizontalRule | BlockKind::ImageEmbed { .. } => false,
            BlockKind::CodeFence { .. } => self.body().is_empty(),
            _ => self.body().is_empty(),
        }
    }

    pub fn is_code_fence(&self) -> bool {
        matches!(self.kind, BlockKind::CodeFence { .. })
    }

    /// Leading indent string for list items (`""` for non-lists).
    pub fn list_indent(&self) -> &str {
        match &self.kind {
            BlockKind::Bullet { indent, .. }
            | BlockKind::Ordered { indent, .. }
            | BlockKind::Task { indent, .. } => indent.as_str(),
            _ => "",
        }
    }

    /// Nesting level for UI padding (tab = 1, every two spaces = 1).
    pub fn list_indent_level(&self) -> usize {
        indent_level(self.list_indent())
    }

    /// Tab / Shift+Tab: nest or un-nest a list item. `None` if not a list or can't outdent.
    pub fn with_indent_delta(&self, deeper: bool) -> Option<Self> {
        let body = self.body();
        match &self.kind {
            BlockKind::Bullet { indent, marker } => {
                let indent = adjust_indent(indent, deeper)?;
                let raw = if body.is_empty() {
                    format!("{indent}{marker} ")
                } else {
                    format!("{indent}{marker} {body}")
                };
                Some(Block::parse_line(&raw))
            }
            BlockKind::Ordered { indent, marker } => {
                let indent = adjust_indent(indent, deeper)?;
                let raw = if body.is_empty() {
                    format!("{indent}{marker} ")
                } else {
                    format!("{indent}{marker} {body}")
                };
                Some(Block::parse_line(&raw))
            }
            BlockKind::Task { indent, checked } => {
                let indent = adjust_indent(indent, deeper)?;
                let mark = if *checked { "x" } else { " " };
                let raw = if body.is_empty() {
                    format!("{indent}- [{mark}] ")
                } else {
                    format!("{indent}- [{mark}] {body}")
                };
                Some(Block::parse_line(&raw))
            }
            _ => None,
        }
    }

    /// Rebuild this block from a new visible body (markers stay structural).
    pub fn with_body(&self, body: &str) -> Self {
        match &self.kind {
            BlockKind::Blank | BlockKind::Paragraph => Block::parse_line(body),
            BlockKind::Heading { level } => {
                let hashes = "#".repeat(*level as usize);
                if body.is_empty() {
                    Block::parse_line(&hashes)
                } else {
                    Block::parse_line(&format!("{hashes} {body}"))
                }
            }
            BlockKind::Bullet { indent, marker } => {
                if body.is_empty() {
                    Block::parse_line(&format!("{indent}{marker} "))
                } else {
                    Block::parse_line(&format!("{indent}{marker} {body}"))
                }
            }
            BlockKind::Ordered { indent, marker } => {
                if body.is_empty() {
                    Block::parse_line(&format!("{indent}{marker} "))
                } else {
                    Block::parse_line(&format!("{indent}{marker} {body}"))
                }
            }
            BlockKind::Task { indent, checked } => {
                let mark = if *checked { "x" } else { " " };
                if body.is_empty() {
                    Block::parse_line(&format!("{indent}- [{mark}] "))
                } else {
                    Block::parse_line(&format!("{indent}- [{mark}] {body}"))
                }
            }
            BlockKind::Blockquote { depth } => {
                let prefix = "> ".repeat(*depth as usize);
                if body.is_empty() {
                    Block::parse_line(prefix.trim_end())
                } else {
                    Block::parse_line(&format!("{prefix}{body}"))
                }
            }
            BlockKind::HorizontalRule => Block {
                kind: BlockKind::HorizontalRule,
                raw: self.raw.clone(),
            },
            BlockKind::ImageEmbed { width, .. } => {
                let raw = match width {
                    Some(w) => format!("![[{body}|{w}]]"),
                    None => format!("![[{body}]]"),
                };
                Block::parse_line(&raw)
            }
            BlockKind::WikiLink { .. } => {
                if body.is_empty() {
                    Block::parse_line("[[")
                } else {
                    Block::parse_line(&format!("[[{body}]]"))
                }
            }
            BlockKind::MdLink { url, .. } => {
                if body.is_empty() {
                    Block::parse_line(&format!("[]({url})"))
                } else {
                    Block::parse_line(&format!("[{body}]({url})"))
                }
            }
            BlockKind::CodeFence { lang } => {
                let open = if lang.is_empty() {
                    "```".to_string()
                } else {
                    format!("```{lang}")
                };
                let raw = format!("{open}\n{body}\n```");
                Block {
                    kind: BlockKind::CodeFence { lang: lang.clone() },
                    raw,
                }
            }
        }
    }
}

/// Split document text into blocks (lines + multi-line code fences).
pub fn parse_blocks(text: &str) -> Vec<Block> {
    if text.is_empty() {
        return vec![Block {
            kind: BlockKind::Blank,
            raw: String::new(),
        }];
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(lang) = fence_open_lang(lines[i]) {
            let start = i;
            i += 1;
            while i < lines.len() && !is_fence_close(lines[i]) {
                i += 1;
            }
            if i < lines.len() {
                i += 1; // include closing ```
            }
            let raw = lines[start..i].join("\n");
            blocks.push(Block {
                kind: BlockKind::CodeFence { lang },
                raw,
            });
            continue;
        }
        blocks.push(Block::parse_line(lines[i]));
        i += 1;
    }

    if blocks.is_empty() {
        blocks.push(Block {
            kind: BlockKind::Blank,
            raw: String::new(),
        });
    }
    blocks
}

/// Replace block `index` with the parse of `new_raw` (may expand into several blocks).
pub fn splice_block_raw(blocks: &mut Vec<Block>, index: usize, new_raw: &str) {
    if index >= blocks.len() {
        return;
    }
    let parsed = parse_blocks(new_raw);
    blocks.splice(index..=index, parsed);
}

pub fn join_blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| b.raw.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn fence_open_lang(line: &str) -> Option<String> {
    let t = line.trim_end();
    let rest = t.strip_prefix("```")?;
    // Opening fence: ``` or ```lang (no other backticks required on this line)
    if rest.contains("```") {
        return None;
    }
    Some(rest.trim().to_string())
}

fn is_fence_close(line: &str) -> bool {
    let t = line.trim();
    t == "```" || (t.starts_with("```") && t.chars().skip(3).all(|c| c == '`'))
}

fn code_fence_body(raw: &str) -> &str {
    let mut lines = raw.split('\n');
    let first = lines.next().unwrap_or("");
    if fence_open_lang(first).is_none() {
        return raw;
    }
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        return "";
    }
    if is_fence_close(rest[rest.len() - 1]) && rest.len() == 1 {
        return "";
    }
    // Can't return joined string as &str easily — store isn't available.
    // Callers that need owned body use code_fence_body_owned.
    // For &str body without alloc when closed: find byte range
    let Some(after_first) = raw.find('\n') else {
        return "";
    };
    let start = after_first + 1;
    if rest.last().is_some_and(|l| is_fence_close(l)) {
        // strip last line
        if let Some(last_nl) = raw.rfind('\n') {
            return &raw[start..last_nl];
        }
    }
    &raw[start..]
}

/// Owned code body (used by the UI).
pub fn code_fence_body_owned(raw: &str) -> String {
    code_fence_body(raw).to_string()
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'#' {
        i += 1;
    }
    if i == 0 || i > 6 {
        return None;
    }
    if i < bytes.len() && bytes[i] != b' ' {
        return None;
    }
    Some((i as u8, line))
}

/// One nesting step written into the source (Tab key).
pub const LIST_INDENT_STEP: &str = "\t";

fn indent_level(indent: &str) -> usize {
    let mut level = 0usize;
    let bytes = indent.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\t' {
            level += 1;
            i += 1;
        } else if bytes[i] == b' ' {
            // Two spaces ≈ one level; a lone space still counts.
            if i + 1 < bytes.len() && bytes[i + 1] == b' ' {
                level += 1;
                i += 2;
            } else {
                level += 1;
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    level
}

fn adjust_indent(indent: &str, deeper: bool) -> Option<String> {
    if deeper {
        return Some(format!("{indent}{LIST_INDENT_STEP}"));
    }
    if indent.is_empty() {
        return None;
    }
    if let Some(rest) = indent.strip_suffix(LIST_INDENT_STEP) {
        return Some(rest.to_string());
    }
    if let Some(rest) = indent.strip_suffix("  ") {
        return Some(rest.to_string());
    }
    if let Some(rest) = indent.strip_suffix(' ') {
        return Some(rest.to_string());
    }
    // Unknown trailing indent char — drop one character.
    let mut chars = indent.chars();
    chars.next();
    Some(chars.as_str().to_string())
}

fn is_horizontal_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let b = t.as_bytes();
    let c = b[0];
    if !matches!(c, b'-' | b'*' | b'_') {
        return false;
    }
    t.bytes().all(|x| x == c || x == b' ') && t.bytes().filter(|&x| x == c).count() >= 3
}

fn parse_task(line: &str) -> Option<(String, bool)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let indent = line[..trimmed_start].to_string();
    let rest = &line[trimmed_start..];
    // `- [ ] ` or `- [x] ` / `- [X] `
    let after_marker = rest
        .strip_prefix("- [")
        .or_else(|| rest.strip_prefix("* ["))?;
    let checked = match after_marker.as_bytes().first()? {
        b' ' | b'x' | b'X' => after_marker.as_bytes()[0] != b' ',
        _ => return None,
    };
    let after_box = after_marker.get(1..)?.strip_prefix(']')?;
    if !after_box.is_empty() && !after_box.starts_with(' ') {
        return None;
    }
    Some((indent, checked))
}

fn parse_blockquote(line: &str) -> Option<u8> {
    let mut depth = 0u8;
    let mut rest = line;
    while let Some(r) = rest.strip_prefix('>') {
        depth = depth.saturating_add(1);
        rest = r.strip_prefix(' ').unwrap_or(r);
        if depth == 6 {
            break;
        }
    }
    if depth == 0 { None } else { Some(depth) }
}

fn parse_bullet(line: &str) -> Option<(String, char, &str)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let indent = line[..trimmed_start].to_string();
    let rest = &line[trimmed_start..];
    let mut chars = rest.chars();
    let marker = chars.next()?;
    if !matches!(marker, '-' | '*' | '+') {
        return None;
    }
    // Require a space after the marker (`- ` → bullet; bare `-` stays a paragraph).
    match chars.next() {
        Some(' ') => Some((indent, marker, rest)),
        _ => None,
    }
}

fn parse_ordered(line: &str) -> Option<(String, String, &str)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let indent = line[..trimmed_start].to_string();
    let rest = &line[trimmed_start..];
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let delim = *bytes.get(i)?;
    if delim != b'.' && delim != b')' {
        return None;
    }
    i += 1;
    // Require a space after `1.` / `1)` so typing the marker alone stays plain text.
    if bytes.get(i) != Some(&b' ') {
        return None;
    }
    let marker = rest[..i].to_string();
    Some((indent, marker, rest))
}

fn parse_wikilink_line(line: &str) -> Option<String> {
    let t = line.trim();
    let inner = t.strip_prefix("[[")?.strip_suffix("]]")?;
    if inner.is_empty() || inner.contains('[') || inner.contains(']') {
        return None;
    }
    // Embeds use ![[…]] — already handled.
    Some(inner.to_string())
}

fn parse_md_link_line(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if !t.starts_with('[') || t.starts_with("![") {
        return None;
    }
    let rest = &t[1..];
    let close = rest.find(']')?;
    let text = &rest[..close];
    let after = &rest[close + 1..];
    let url = after.strip_prefix('(')?.strip_suffix(')')?;
    if text.is_empty() || url.is_empty() {
        return None;
    }
    // Whole line must be exactly the link.
    if t != format!("[{text}]({url})") {
        return None;
    }
    Some((text.to_string(), url.to_string()))
}

fn parse_image_embed(line: &str) -> Option<(String, Option<u32>)> {
    let t = line.trim();
    let inner = t.strip_prefix("![[")?.strip_suffix("]]")?;
    let (name, width) = if let Some((n, w)) = inner.rsplit_once('|') {
        let w = w.trim();
        if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) {
            (n.trim(), w.parse().ok())
        } else {
            (inner.trim(), None)
        }
    } else {
        (inner.trim(), None)
    };
    let name = name.strip_prefix("attachments/").unwrap_or(name);
    if name.is_empty() {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    if !(lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif"))
    {
        return None;
    }
    Some((name.to_string(), width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_task_quote_hr() {
        let t = Block::parse_line("- [ ] buy milk");
        assert!(matches!(t.kind, BlockKind::Task { checked: false, .. }));
        assert_eq!(t.body(), "buy milk");
        let t2 = Block::parse_line("- [x] done");
        assert!(matches!(t2.kind, BlockKind::Task { checked: true, .. }));
        let q = Block::parse_line("> hello");
        assert!(matches!(q.kind, BlockKind::Blockquote { depth: 1 }));
        assert_eq!(q.body(), "hello");
        let hr = Block::parse_line("---");
        assert!(matches!(hr.kind, BlockKind::HorizontalRule));
    }

    #[test]
    fn parses_bullet_and_heading() {
        let b = Block::parse_line("- hello");
        assert!(matches!(b.kind, BlockKind::Bullet { marker: '-', .. }));
        assert_eq!(b.body(), "hello");

        let h = Block::parse_line("## Title");
        assert!(matches!(h.kind, BlockKind::Heading { level: 2 }));
        assert_eq!(h.body(), "Title");
    }

    #[test]
    fn parses_image_embed() {
        let b = Block::parse_line("![[Pasted image 1.png|400]]");
        assert!(matches!(
            b.kind,
            BlockKind::ImageEmbed {
                width: Some(400),
                ..
            }
        ));
    }

    #[test]
    fn parses_code_fence() {
        let text = "intro\n```rust\nfn main() {}\n```\nafter\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 4);
        assert!(matches!(
            &blocks[1].kind,
            BlockKind::CodeFence { lang } if lang == "rust"
        ));
        assert_eq!(blocks[1].body(), "fn main() {}");
        assert_eq!(join_blocks(&blocks), text);
    }

    #[test]
    fn roundtrip_join() {
        let text = "# Hi\n\n- a\n- b\n";
        let blocks = parse_blocks(text);
        assert_eq!(join_blocks(&blocks), text);
    }

    #[test]
    fn with_body_keeps_bullet_marker() {
        let b = Block::parse_line("- hello");
        let b2 = b.with_body("world");
        assert!(matches!(b2.kind, BlockKind::Bullet { marker: '-', .. }));
        assert_eq!(b2.raw, "- world");
        assert_eq!(b2.body(), "world");
    }

    #[test]
    fn parses_wikilink_and_md_link() {
        let w = Block::parse_line("[[Other Note]]");
        assert!(matches!(w.kind, BlockKind::WikiLink { .. }));
        assert_eq!(w.body(), "Other Note");

        let m = Block::parse_line("[label](https://example.com)");
        assert!(matches!(m.kind, BlockKind::MdLink { .. }));
        assert_eq!(m.body(), "label");
    }

    #[test]
    fn bare_marker_stays_paragraph_until_space() {
        assert!(matches!(Block::parse_line("-").kind, BlockKind::Paragraph));
        assert!(matches!(
            Block::parse_line("- ").kind,
            BlockKind::Bullet { .. }
        ));
        assert!(matches!(Block::parse_line("1)").kind, BlockKind::Paragraph));
        assert!(matches!(
            Block::parse_line("1) ").kind,
            BlockKind::Ordered { .. }
        ));
        let dot = Block::parse_line("1. Go");
        assert!(matches!(dot.kind, BlockKind::Ordered { .. }));
        assert_eq!(dot.body(), "Go");
        let paren = Block::parse_line("1) Go");
        assert!(matches!(paren.kind, BlockKind::Ordered { .. }));
        assert_eq!(paren.body(), "Go");
        assert!(matches!(
            Block::parse_line("10. Prelude").kind,
            BlockKind::Ordered { .. }
        ));
    }

    #[test]
    fn tab_nests_and_unnests_bullet() {
        let b = Block::parse_line("- hello");
        let nested = b.with_indent_delta(true).unwrap();
        assert_eq!(nested.raw, "\t- hello");
        assert_eq!(nested.list_indent_level(), 1);
        let back = nested.with_indent_delta(false).unwrap();
        assert_eq!(back.raw, "- hello");
        assert!(back.with_indent_delta(false).is_none());
    }
}
