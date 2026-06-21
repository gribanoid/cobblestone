// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use cobblestone_core::{Note, Store};
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

type AppState = Arc<Store>;

pub async fn run(store: Store, port: u16) -> Result<()> {
    let state = Arc::new(store);
    let ui_dist = ui_dist_dir();

    let api = Router::new()
        .route("/notes", put(api_create))
        .route("/notes/{slug}", get(api_get))
        .route("/notes/{slug}", post(api_update))
        .route("/notes/{slug}", delete(api_delete))
        .route("/notes/{slug}/graph", get(api_graph))
        .route("/notes/move", post(api_move_note))
        .route("/tree", get(api_tree))
        .route("/folders", post(api_create_folder))
        .route("/folders/move", post(api_move_folder))
        .route("/folders/rename", post(api_rename_folder))
        .route("/folders/delete", post(api_delete_folder))
        .route("/notes/rename", post(api_rename_note))
        .route("/search", get(api_search));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(&ui_dist).fallback(ServeFile::new(ui_dist.join("index.html"))))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let url = format!("http://{addr}");

    if !ui_dist.join("index.html").exists() {
        eprintln!(
            "Web UI not built — run `npm run build:web` from the repo root first.\n\
             API is available at {url}/api/…"
        );
    } else {
        println!("Cobblestone web  →  {url}");
    }
    println!("Press Ctrl+C to stop.");

    let _ = open::that(&url);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn ui_dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/apps/web/dist")
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct NoteInfo {
    slug: String,
    title: String,
    created: String,
    modified: String,
    size: u64,
    preview: String,
    tags: Vec<String>,
}

impl From<&Note> for NoteInfo {
    fn from(n: &Note) -> Self {
        Self {
            slug: n.name.clone(),
            title: n.title.clone(),
            created: n.created.clone(),
            modified: n.modified.clone(),
            size: n.size,
            preview: n.preview.clone(),
            tags: n.tags.clone(),
        }
    }
}

#[derive(Serialize)]
struct NoteContent {
    slug: String,
    title: String,
    content: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn api_tree(State(store): State<AppState>) -> impl IntoResponse {
    match store.list_tree() {
        Ok(tree) => (StatusCode::OK, Json(tree)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_get(
    State(store): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match store.read(&slug) {
        Ok(content) => {
            let title = title_from_content(&content, &slug);
            Json(NoteContent { slug, title, content }).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Note not found").into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateBody {
    content: String,
}

async fn api_update(
    State(store): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<UpdateBody>,
) -> impl IntoResponse {
    match store.write(&slug, &body.content) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateBody {
    title: String,
    folder: Option<String>,
}

async fn api_create(
    State(store): State<AppState>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    let slug = match store.note_id_from_title(body.folder.as_deref(), &body.title) {
        Ok(s) => s,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if store.exists(&slug) {
        return (StatusCode::CONFLICT, "Note already exists").into_response();
    }
    let content = format!("# {}\n\n", body.title);
    match store.write(&slug, &content) {
        Ok(_) => Json(serde_json::json!({ "slug": slug })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_delete(
    State(store): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match store.delete(&slug) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Note not found").into_response(),
    }
}

#[derive(Deserialize)]
struct FolderBody {
    path: String,
}

async fn api_create_folder(
    State(store): State<AppState>,
    Json(body): Json<FolderBody>,
) -> impl IntoResponse {
    match store.create_folder(&body.path) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct MoveNoteBody {
    slug: String,
    folder: Option<String>,
}

async fn api_move_note(
    State(store): State<AppState>,
    Json(body): Json<MoveNoteBody>,
) -> impl IntoResponse {
    match store.move_note(&body.slug, body.folder.as_deref()) {
        Ok(slug) => Json(serde_json::json!({ "slug": slug })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct MoveFolderBody {
    path: String,
    dest_parent: Option<String>,
}

async fn api_move_folder(
    State(store): State<AppState>,
    Json(body): Json<MoveFolderBody>,
) -> impl IntoResponse {
    match store.move_folder(&body.path, body.dest_parent.as_deref()) {
        Ok(path) => Json(serde_json::json!({ "path": path })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct RenameNoteBody {
    slug: String,
    title: String,
}

async fn api_rename_note(
    State(store): State<AppState>,
    Json(body): Json<RenameNoteBody>,
) -> impl IntoResponse {
    match store.rename_note(&body.slug, &body.title) {
        Ok(slug) => Json(serde_json::json!({ "slug": slug })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct RenameFolderBody {
    path: String,
    name: String,
}

async fn api_rename_folder(
    State(store): State<AppState>,
    Json(body): Json<RenameFolderBody>,
) -> impl IntoResponse {
    match store.rename_folder(&body.path, &body.name) {
        Ok(path) => Json(serde_json::json!({ "path": path })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct DeleteFolderBody {
    path: String,
}

async fn api_delete_folder(
    State(store): State<AppState>,
    Json(body): Json<DeleteFolderBody>,
) -> impl IntoResponse {
    match store.delete_folder(&body.path) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

async fn api_search(
    State(store): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    match store.search(&q.query) {
        Ok(results) => {
            let list: Vec<NoteInfo> = results.iter().map(|(n, _)| NoteInfo::from(n)).collect();
            (StatusCode::OK, Json(list)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_graph(
    State(store): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match store.note_graph(&slug) {
        Ok(graph) => Json(graph).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn title_from_content(content: &str, fallback: &str) -> String {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").to_string())
        .unwrap_or_else(|| fallback.to_string())
}
