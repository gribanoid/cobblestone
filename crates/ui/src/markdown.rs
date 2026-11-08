// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd, html};

use crate::util::body_for_preview;

/// Render note body to sanitized HTML.
///
/// `image_src` maps an embed target (`Pasted image….png` or `attachments/…`)
/// to a browser-usable `src` (data URL or `/api/attachments/…`).
///
/// `resolve_wikilink` maps a wikilink target (title or note id) to a note slug
/// for clickable Reading-view links. Pass `|_| None` to leave `[[…]]` as text.
pub fn render_markdown_preview(
    content: &str,
    title: &str,
    image_src: &dyn Fn(&str) -> Option<String>,
) -> String {
    render_markdown_preview_with_links(content, title, image_src, &|_| None)
}

/// Like [`render_markdown_preview`], with wikilink resolution.
pub fn render_markdown_preview_with_links(
    content: &str,
    title: &str,
    image_src: &dyn Fn(&str) -> Option<String>,
    resolve_wikilink: &dyn Fn(&str) -> Option<String>,
) -> String {
    let body = body_for_preview(content, title);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_WIKILINKS;
    let mut parser = Parser::new_ext(&body, options).into_offset_iter();
    let events = std::iter::from_fn(|| {
        let (event, range) = parser.next()?;
        let raw = &body[range];
        let (end, replacement) = match event {
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                ..
            }) => {
                let inner = raw
                    .strip_prefix("[[")
                    .and_then(|text| text.strip_suffix("]]"));
                let replacement = inner.and_then(|text| {
                    let (target, label) = split_wikilink_target(text);
                    let slug = resolve_wikilink(target)?;
                    Some(format!(
                        r##"<a class="cb-wikilink" href="#" data-slug="{}">{}</a>"##,
                        html_escape_attr(&slug),
                        html_escape_text(label),
                    ))
                });
                (TagEnd::Link, replacement)
            }
            Event::Start(Tag::Image {
                link_type: LinkType::WikiLink { .. },
                ..
            }) => {
                let replacement = raw
                    .strip_prefix("![[")
                    .and_then(|text| text.strip_suffix("]]"))
                    .and_then(|text| render_image_embed(text, image_src));
                (TagEnd::Image, replacement)
            }
            _ => return Some(event),
        };
        for (next, _) in parser.by_ref() {
            if next == Event::End(end) {
                break;
            }
        }
        Some(match replacement {
            Some(html) => Event::InlineHtml(html.into()),
            None => Event::Text(raw.into()),
        })
    });
    let mut html_out = String::new();
    html::push_html(&mut html_out, events);
    clean_html(&html_out)
}

fn split_wikilink_target(inner: &str) -> (&str, &str) {
    let inner = inner.trim();
    // Drop heading / block anchors for navigation: Note#heading → Note
    let target_part = inner.split_once('|').map(|(t, _)| t).unwrap_or(inner);
    let target = target_part
        .split(['#', '^'])
        .next()
        .unwrap_or(target_part)
        .trim();
    let label = if let Some((_, alias)) = inner.split_once('|') {
        alias.trim()
    } else {
        target
    };
    (target, if label.is_empty() { target } else { label })
}

fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn render_image_embed(inner: &str, image_src: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    let (name, width) = split_embed_target(inner);
    let name = name.strip_prefix("attachments/").unwrap_or(name);
    if !is_image_name(name) {
        return None;
    }
    let src = image_src(name)?;
    let width = width.map_or_else(String::new, |width| format!(r#" width="{width}""#));
    Some(format!(
        r#"<img src="{}" alt="{}"{} class="cb-embed-image" />"#,
        html_escape_attr(&src),
        html_escape_attr(name),
        width,
    ))
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
    static SCHEMES: LazyLock<HashSet<&'static str>> =
        LazyLock::new(|| ["http", "https", "data", "mailto"].into_iter().collect());
    ammonia::Builder::default()
        .add_tags(["img"])
        .add_tag_attributes("img", ["src", "alt", "width", "height", "class"])
        .add_tag_attributes("a", ["href", "class", "data-slug", "title"])
        .add_allowed_classes("code", ["language-tasks"])
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
        let html = render_markdown_preview("![[Photo.png|400]]\n", "T", &|n| {
            assert_eq!(n, "Photo.png");
            Some("/api/attachments/Photo.png".into())
        });
        assert!(html.contains("<img"));
        assert!(html.contains("Photo.png"));
        assert!(html.contains("width=\"400\""));
    }

    #[test]
    fn expands_clickable_wikilink() {
        let html = render_markdown_preview_with_links(
            "See [[Hello World|hi]] and [[Missing]]. Code: `[[Hello World]]`.\n\n```tasks\nnot done\n```\n",
            "T",
            &|_| None,
            &|t| {
                if t.eq_ignore_ascii_case("Hello World") {
                    Some("hello-world".into())
                } else {
                    None
                }
            },
        );
        assert!(html.contains("data-slug=\"hello-world\""));
        assert!(html.contains(">hi</a>"));
        assert!(html.contains("[[Missing]]"));
        assert!(html.contains("<code>[[Hello World]]</code>"));
        assert!(html.contains(r#"<pre><code class="language-tasks">"#));
    }
}
