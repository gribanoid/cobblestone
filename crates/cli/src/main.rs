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
/// Notes are stored as Markdown files in ~/Documents/CobblestoneVault.
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
        /// Optional sub-path inside the vault to list
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
    /// Backup the vault to a zip archive
    Backup {
        /// Output zip path, or a directory to write into
        /// (default: ~/Documents/CobblestoneVault-backups/CobblestoneVault-TIMESTAMP.zip)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// Restore the vault from a backup zip
    Restore {
        /// Path to a zip created by `cb backup`
        archive: std::path::PathBuf,
        /// Overwrite existing files that conflict with the archive
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Start the web interface
    Web {
        /// Port to listen on (default: 3000)
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    /// Open or create today's daily note (Periodic Notes)
    Today,
    /// Manage first-party plugins
    Plugin {
        #[command(subcommand)]
        action: PluginCmd,
    },
    /// Manage projects (vault folders)
    Project {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    /// Ranked Omni search across the vault
    Omni {
        /// Search query
        query: String,
        /// Max results
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum PluginCmd {
    /// List built-in plugins and enable state
    List,
    /// Enable a plugin by id
    Enable { id: String },
    /// Disable a plugin by id
    Disable { id: String },
    /// List registered plugin commands
    Commands,
    /// Run a plugin command by id
    Run { id: String },
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// List registered projects
    List,
    /// Register a folder as a project
    Add {
        name: String,
        path: std::path::PathBuf,
    },
    /// Open a project (sets last_opened; next commands use it)
    Open { name: String },
    /// Remove a project from the registry
    Rm { name: String },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.interactive {
        return interactive::run(storage::Store::init()?);
    }
    let command = match cli.command {
        Some(Command::Project { action }) => {
            return match action {
                ProjectCmd::List => commands::cmd_project_list(),
                ProjectCmd::Add { name, path } => commands::cmd_project_add(&name, &path),
                ProjectCmd::Open { name } => commands::cmd_project_open(&name),
                ProjectCmd::Rm { name } => commands::cmd_project_rm(&name),
            };
        }
        command => command,
    };
    let store = storage::Store::init()?;

    match command {
        None | Some(Command::Ls { path: None }) => list::run(&store),
        Some(Command::Ls { path: Some(p) }) => list::run_path(&store, &p),
        Some(Command::New { title }) => commands::cmd_new(&store, &title),
        Some(Command::Show { name }) => commands::cmd_show(&store, &name),
        Some(Command::Edit { name }) => commands::cmd_edit(&store, &name),
        Some(Command::Rm { name }) => commands::cmd_delete(&store, &name),
        Some(Command::Search { query }) => list::search(&store, &query),
        Some(Command::Backup { output }) => commands::cmd_backup(&store, output),
        Some(Command::Restore { archive, force }) => commands::cmd_restore(&store, &archive, force),
        Some(Command::Web { port }) => web::run(store, port).await,
        Some(Command::Today) => commands::cmd_today(&store),
        Some(Command::Plugin { action }) => match action {
            PluginCmd::List => commands::cmd_plugin_list(&store),
            PluginCmd::Enable { id } => commands::cmd_plugin_enable(&store, &id, true),
            PluginCmd::Disable { id } => commands::cmd_plugin_enable(&store, &id, false),
            PluginCmd::Commands => commands::cmd_plugin_commands(&store),
            PluginCmd::Run { id } => commands::cmd_plugin_run(&store, &id),
        },
        Some(Command::Project { .. }) => unreachable!(),
        Some(Command::Omni { query, limit }) => commands::cmd_omni(&store, &query, limit),
    }
}
