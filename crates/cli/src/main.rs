// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod interactive;
mod list;
mod web;

use cobblestone_core as storage;

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

/// Cobblestone — open-source knowledge base for your private thoughts.
///
/// Notes are stored as Markdown files in ~/.cobblestone.
/// Run `cb -i` for an interactive TUI, or `cb web` to open the browser UI.
#[derive(Parser)]
#[command(name = "cb", version, about, long_about = None)]
struct Cli {
    /// Launch interactive TUI mode
    #[arg(short = 'i', long = "interactive", global = false)]
    interactive: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// List all notes (default when no subcommand is given)
    Ls {
        /// Optional sub-path inside ~/.cobblestone to list
        path: Option<String>,
    },
    /// Create a new note (opens $EDITOR)
    New {
        /// Title of the new note
        title: String,
    },
    /// Print a note to stdout with basic Markdown rendering
    Show {
        /// Note slug (filename without .md)
        name: String,
    },
    /// Open a note in $EDITOR
    Edit {
        /// Note slug
        name: String,
    },
    /// Delete a note
    Rm {
        /// Note slug
        name: String,
    },
    /// Search note contents
    Search {
        /// Search query
        query: String,
    },
    /// Start the web interface
    Web {
        /// Port to listen on (default: 3000)
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli   = Cli::parse();
    let store = storage::Store::init()?;

    if cli.interactive {
        return interactive::run(store);
    }

    match cli.command {
        None | Some(Command::Ls { path: None }) => {
            list::run(&store)
        }
        Some(Command::Ls { path: Some(p) }) => {
            list::run_path(&store, &p)
        }
        Some(Command::New   { title }) => commands::cmd_new(&store, &title),
        Some(Command::Show  { name  }) => commands::cmd_show(&store, &name),
        Some(Command::Edit  { name  }) => commands::cmd_edit(&store, &name),
        Some(Command::Rm    { name  }) => commands::cmd_delete(&store, &name),
        Some(Command::Search{ query }) => list::search(&store, &query),
        Some(Command::Web   { port  }) => web::run(store, port).await,
    }
}
