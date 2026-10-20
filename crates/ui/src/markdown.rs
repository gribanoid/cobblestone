// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use pulldown_cmark::{Options, Parser, html};

use crate::util::body_for_preview;

/// Render note body to sanitized HTML.
///
/// `image_src` maps an embed target (`Pasted image….png` or `attachments/…`)
/// to a browser-usable `src` (data URL or `/api/attachments/…`).
pub fn render_markdown_preview(
    content: &str,
    title: &str,
    image_src: &dyn Fn(&str) -> Option<String>,
) -> String {
    let body = body_for_preview(content, title);
    let body = expand_image_embeds(&body, image_src);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(&body, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    clean_html(&html_out)
}

/// Turn Obsidian-style `![[file|width]]` / `![[file]]` into `<img>` tags.
fn expand_image_embeds(body: &str, image_src: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("![[") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 3..];
        let Some(end) = after.find("]]") else {
            out.push_str("![[");
            rest = after;
            continue;
        };
        let inner = &after[..end];
        let (name, width) = split_embed_target(inner);
        let resolved_name = name
            .strip_prefix("attachments/")
            .unwrap_or(name);
        if is_image_name(resolved_name) {
            if let Some(src) = image_src(resolved_name) {
                out.push_str("<img src=\"");
                out.push_str(&html_escape_attr(&src));
                out.push_str("\" alt=\"");
                out.push_str(&html_escape_attr(resolved_name));
                out.push('"');
                if let Some(w) = width {
                    out.push_str(" width=\"");
                    out.push_str(w);
                    out.push('"');
                }
                out.push_str(" class=\"cb-embed-image\" />");
                rest = &after[end + 2..];
                continue;
            }
        }
        out.push_str("![[");
        out.push_str(inner);
        out.push_str("]]");
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn split_embed_target(inner: &str) -> (&str, Option<&str>) {
    let inner = inner.trim();
    if let Some((name, width)) = inner.rsplit_once('|') {
        let width = width.trim();
        if !width.is_empty() && width.chars().all(|c| c.is_ascii_digit()) {
            return (name.trim(), Some(width));
        }
    }
    (inner, None)
}

fn is_image_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".svg")
        || lower.ends_with(".avif")
}

fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn clean_html(html: &str) -> String {
    use std::sync::LazyLock;
    static SCHEMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
        ["http", "https", "data", "mailto"].into_iter().collect()
    });
    ammonia::Builder::default()
        .add_tags(["img"])
        .add_tag_attributes("img", ["src", "alt", "width", "height", "class"])
        .url_schemes(SCHEMES.clone())
        .clean(html)
        .to_string()
}

/// Collect bare attachment filenames referenced by `![[…]]` embeds.
pub fn embed_image_names(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("![[") {
        let after = &rest[start + 3..];
        let Some(end) = after.find("]]") else { break };
        let inner = &after[..end];
        if let Some(nl) = inner.find('\n') {
            rest = &after[nl..];
            continue;
        }
        let (name, _) = split_embed_target(inner);
        let resolved = name.strip_prefix("attachments/").unwrap_or(name);
        if is_image_name(resolved) {
            names.push(resolved.to_string());
        }
        rest = &after[end + 2..];
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_wikilink_image() {
        let html = render_markdown_preview(
            "![[Photo.png|400]]\n",
            "T",
            &|n| {
                assert_eq!(n, "Photo.png");
                Some("/api/attachments/Photo.png".into())
            },
        );
        assert!(html.contains("<img"));
        assert!(html.contains("Photo.png"));
        assert!(html.contains("width=\"400\""));
    }
}
