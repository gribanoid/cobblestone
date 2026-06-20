// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{self, Write};

use anyhow::{bail, Context, Result};
use chrono::Local;
use cobblestone_core::{slugify, Store};

pub fn cmd_new(store: &Store, title: &str) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        bail!("Note title cannot be empty.");
    }
    let name = slugify(title);
    if store.exists(&name) {
        bail!("Note '{}' already exists. Use `cb edit {}` to edit it.", name, name);
    }
    let date = Local::now().format("%Y-%m-%d").to_string();
    let content = format!("# {title}\n\n*Created: {date}*\n\n");
    store.write(&name, &content)?;

    println!("Created: {}", store.path_for(&name).display());

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor)
        .arg(store.path_for(&name))
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;
    Ok(())
}

pub fn cmd_show(store: &Store, name: &str) -> Result<()> {
    let content = store.read(name)?;
    render_to_terminal(&content);
    Ok(())
}

pub fn cmd_edit(store: &Store, name: &str) -> Result<()> {
    if !store.exists(name) {
        bail!("Note '{}' does not exist. Use `cb new` to create it.", name);
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor)
        .arg(store.path_for(name))
        .status()
        .with_context(|| format!("Failed to launch editor '{editor}'"))?;
    Ok(())
}

pub fn cmd_delete(store: &Store, name: &str) -> Result<()> {
    if !store.exists(name) {
        bail!("Note '{}' does not exist.", name);
    }
    print!("Delete '{name}'? [y/N] ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        store.delete(name)?;
        println!("Deleted.");
    } else {
        println!("Aborted.");
    }
    Ok(())
}

/// Simple Markdown → ANSI terminal renderer.
pub fn render_to_terminal(content: &str) {
    for line in content.lines() {
        if let Some(h) = line.strip_prefix("# ") {
            println!("\x1b[1;33m{h}\x1b[0m");
        } else if let Some(h) = line.strip_prefix("## ") {
            println!("\x1b[1;36m{h}\x1b[0m");
        } else if let Some(h) = line.strip_prefix("### ") {
            println!("\x1b[1;32m{h}\x1b[0m");
        } else if let Some(t) = line.strip_prefix("- [ ] ") {
            println!("  \x1b[31m☐\x1b[0m  {t}");
        } else if let Some(t) = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
        {
            println!("  \x1b[32m✓\x1b[0m  \x1b[9m{t}\x1b[0m");
        } else if let Some(t) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
        {
            println!("  \x1b[34m•\x1b[0m  {t}");
        } else if let Some(t) = line.strip_prefix("> ") {
            println!("  \x1b[90m│\x1b[0m  \x1b[3m{t}\x1b[0m");
        } else {
            println!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_does_not_panic_on_empty() {
        render_to_terminal("");
    }

    #[test]
    fn render_does_not_panic_on_all_variants() {
        let md = "# H1\n## H2\n### H3\n- [ ] todo\n- [x] done\n- item\n* star\n> quote\nplain";
        render_to_terminal(md);
    }
}
