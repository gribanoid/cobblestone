// SPDX-License-Identifier: MIT

//! Serialized app operations — mirrors the former TypeScript async queue.
//! Driven by [`use_coroutine`] so vault mutations never interleave.

use dioxus::prelude::*;
use futures::StreamExt;

use crate::actions::{
    apply_drop, confirm_delete, confirm_rename, create_folder, create_note, flush_save,
    move_note_to_folder, open_note, reload_tree, run_search, switch_vault, toggle_reading,
};
use crate::backend::SharedBackend;
use crate::state::AppCtx;

#[cfg(feature = "desktop")]
use cobblestone_plugin_host::SharedPluginHost;

#[cfg(feature = "desktop")]
use crate::actions::{confirm_plugin_prompt, run_plugin_command};

#[derive(Clone, Debug)]
pub enum AppOp {
    ReloadTree,
    Search,
    OpenNote(String),
    CreateNote {
        title: String,
        folder: Option<String>,
    },
    CreateFolder {
        name: String,
        parent: Option<String>,
    },
    ConfirmRename(String),
    ConfirmDelete,
    Drop {
        dest: Option<String>,
    },
    MoveNote {
        slug: String,
        folder: Option<String>,
    },
    Save,
    SwitchVault(String),
    ToggleReading,
    /// Run a first-party plugin command id (desktop host).
    RunPluginCommand(String),
    /// Finish a PromptValue modal and run the pending plugin command.
    ConfirmPluginPrompt {
        command: String,
        value: String,
    },
}

pub type Ops = Coroutine<AppOp>;

pub fn spawn_ops(ctx: AppCtx, backend: SharedBackend) -> Ops {
    #[cfg(feature = "desktop")]
    let host = try_use_context::<SharedPluginHost>();
    #[cfg(not(feature = "desktop"))]
    let host: Option<()> = None;
    let _ = host;

    use_coroutine(move |mut rx: UnboundedReceiver<AppOp>| {
        let backend = backend.clone();
        #[cfg(feature = "desktop")]
        let host = host.clone();
        async move {
            while let Some(op) = rx.next().await {
                #[cfg(feature = "desktop")]
                dispatch(ctx, backend.clone(), host.clone(), op).await;
                #[cfg(not(feature = "desktop"))]
                dispatch(ctx, backend.clone(), op).await;
            }
        }
    })
}

#[cfg(feature = "desktop")]
async fn dispatch(ctx: AppCtx, backend: SharedBackend, host: Option<SharedPluginHost>, op: AppOp) {
    match op {
        AppOp::ReloadTree => reload_tree(ctx, backend).await,
        AppOp::Search => run_search(ctx, backend, host.clone()).await,
        AppOp::OpenNote(slug) => open_note(ctx, backend, slug).await,
        AppOp::CreateNote { title, folder } => create_note(ctx, backend, title, folder).await,
        AppOp::CreateFolder { name, parent } => create_folder(ctx, backend, name, parent).await,
        AppOp::ConfirmRename(value) => confirm_rename(ctx, backend, value).await,
        AppOp::ConfirmDelete => confirm_delete(ctx, backend).await,
        AppOp::Drop { dest } => apply_drop(ctx, backend, dest).await,
        AppOp::MoveNote { slug, folder } => move_note_to_folder(ctx, backend, slug, folder).await,
        AppOp::Save => {
            let _ = flush_save(ctx, backend).await;
        }
        AppOp::SwitchVault(path) => switch_vault(ctx, backend, path).await,
        AppOp::ToggleReading => toggle_reading(ctx),
        AppOp::RunPluginCommand(id) => run_plugin_command(ctx, backend, host, id).await,
        AppOp::ConfirmPluginPrompt { command, value } => {
            confirm_plugin_prompt(ctx, backend, host, command, value).await
        }
    }
}

#[cfg(not(feature = "desktop"))]
async fn dispatch(ctx: AppCtx, backend: SharedBackend, op: AppOp) {
    match op {
        AppOp::ReloadTree => reload_tree(ctx, backend).await,
        AppOp::Search => run_search(ctx, backend).await,
        AppOp::OpenNote(slug) => open_note(ctx, backend, slug).await,
        AppOp::CreateNote { title, folder } => create_note(ctx, backend, title, folder).await,
        AppOp::CreateFolder { name, parent } => create_folder(ctx, backend, name, parent).await,
        AppOp::ConfirmRename(value) => confirm_rename(ctx, backend, value).await,
        AppOp::ConfirmDelete => confirm_delete(ctx, backend).await,
        AppOp::Drop { dest } => apply_drop(ctx, backend, dest).await,
        AppOp::MoveNote { slug, folder } => move_note_to_folder(ctx, backend, slug, folder).await,
        AppOp::Save => {
            let _ = flush_save(ctx, backend).await;
        }
        AppOp::SwitchVault(path) => switch_vault(ctx, backend, path).await,
        AppOp::ToggleReading => toggle_reading(ctx),
        AppOp::RunPluginCommand(_) | AppOp::ConfirmPluginPrompt { .. } => {
            let mut ctx = ctx;
            ctx.show_error("Plugins run in the desktop app / CLI only");
        }
    }
}
