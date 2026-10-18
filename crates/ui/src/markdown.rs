// SPDX-License-Identifier: MIT

use pulldown_cmark::{Options, Parser, html};

use crate::util::body_for_preview;

pub fn render_markdown_preview(content: &str, title: &str) -> String {
    let body = body_for_preview(content, title);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(&body, options);
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    ammonia::clean(&html_out)
}
