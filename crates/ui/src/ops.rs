// SPDX-License-Identifier: MIT

//! Serialized app operations — mirrors the former TypeScript async queue.
//! Driven by [`use_coroutine`] so vault mutations never interleave.

use dioxus::prelude::*;
use futures::StreamExt;

use crate::actions::{
    apply_drop, confirm_delete, confirm_rename, create_folder, create_note, flush_save,
    move_note_to_folder, open_note, reload_tree, run_search,
};
use crate::backend::SharedBackend;
use crate::state::AppCtx;

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
}

pub type Ops = Coroutine<AppOp>;

pub fn spawn_ops(ctx: AppCtx, backend: SharedBackend) -> Ops {
    use_coroutine(move |mut rx: UnboundedReceiver<AppOp>| {
        let backend = backend.clone();
        async move {
            while let Some(op) = rx.next().await {
                dispatch(ctx, backend.clone(), op).await;
            }
        }
    })
}

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
    }
}
