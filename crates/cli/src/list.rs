// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use colored::Colorize;

use cobblestone_core::Store;

pub fn run(store: &Store) -> Result<()> {
    let notes = store.list_notes()?;

    let path = store.root.display().to_string();
    println!("{}", format!(" 🪨  {path}").bold());
    println!();

    if notes.is_empty() {
        println!(
            "{}",
            "  No notes yet.\n  Run `cb new \"My First Note\"` to get started.".dimmed()
        );
        return Ok(());
    }

    for note in &notes {
        // Title + date
        println!(
            "  {}  {}",
            note.title.yellow().bold(),
            note.modified.dimmed()
        );

        // Preview
        if !note.preview.is_empty() {
            println!("     {}", note.preview.dimmed());
        }

        // Tags
        if !note.tags.is_empty() {
            let tag_line = note
                .tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join("  ");
            println!("     {}", tag_line.cyan().dimmed());
        }

        println!();
    }

    println!(
        "{}",
        format!("  {} note(s)  ·  cb -i for interactive mode  ·  cb web to open browser",
                notes.len()).dimmed()
    );

    Ok(())
}

pub fn run_path(store: &Store, path: &str) -> Result<()> {
    // If a specific sub-path is given, list the real filesystem entries there;
    // otherwise fall back to the notes list.
    if path.is_empty() {
        return run(store);
    }

    let target = if path.starts_with('/') || path.starts_with('~') {
        let home = dirs::home_dir()
            .context("Cannot locate home directory")?
            .display()
            .to_string();
        std::path::PathBuf::from(path.replacen('~', &home, 1))
    } else {
        store.root.join(path)
    };

    if !target.exists() {
        anyhow::bail!("Path does not exist: {}", target.display());
    }

    println!("{}", format!(" 🪨  {}", target.display()).bold());
    println!();

    for entry in std::fs::read_dir(&target)? {
        let entry = entry?;
        let meta  = entry.metadata()?;
        let name  = entry.file_name().to_string_lossy().to_string();

        if meta.is_dir() {
            println!("  {}  {}", "d".cyan(), name.bold());
        } else {
            let size = human_size(meta.len());
            println!("  {}  {}  {}", "f".dimmed(), name, size.dimmed());
        }
    }

    Ok(())
}

pub fn search(store: &Store, query: &str) -> Result<()> {
    let results = store.search(query)?;

    if results.is_empty() {
        println!("{}", format!("No results for \"{query}\"").dimmed());
        return Ok(());
    }

    println!("{}", format!(" \u{1F50D}  Results for \"{query}\"").bold());
    println!();

    for (note, lines) in &results {
        println!("  {}", note.title.yellow().bold());
        for line in lines {
            println!("     {}", line.dimmed());
        }
        println!();
    }

    Ok(())
}

fn human_size(bytes: u64) -> String {
    match bytes {
        b if b < 1024       => format!("{b} B"),
        b if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.0),
        b                   => format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)),
    }
}
