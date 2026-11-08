// SPDX-License-Identifier: MIT

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
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
        .route("/files/delete", post(api_delete_file))
        .route("/notes/rename", post(api_rename_note))
        .route("/search", get(api_search))
        .route("/attachments", post(api_upload_attachment))
        .route("/attachments/{*name}", get(api_get_attachment));

    let app = Router::new()
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(&ui_dist).fallback(ServeFile::new(ui_dist.join("index.html"))),
        )
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let url = format!("http://{addr}");

    if !ui_dist.join("index.html").exists() {
        eprintln!(
            "Web UI not built — run `make web-build` from the repo root first.\n\
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/ui/dist-web")
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

async fn api_get(State(store): State<AppState>, Path(slug): Path<String>) -> impl IntoResponse {
    match store.read(&slug) {
        Ok(content) => {
            let title = title_from_content(&content, &slug);
            Json(NoteContent {
                slug,
                title,
                content,
            })
            .into_response()
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

async fn api_delete(State(store): State<AppState>, Path(slug): Path<String>) -> impl IntoResponse {
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

async fn api_delete_file(
    State(store): State<AppState>,
    Json(body): Json<DeleteFolderBody>,
) -> impl IntoResponse {
    match store.delete_file(&body.path) {
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

async fn api_graph(State(store): State<AppState>, Path(slug): Path<String>) -> impl IntoResponse {
    match store.note_graph(&slug) {
        Ok(graph) => Json(graph).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct UploadAttachmentBody {
    #[serde(default)]
    folder: Option<String>,
    filename: String,
    data_base64: String,
}

async fn api_upload_attachment(
    State(store): State<AppState>,
    Json(body): Json<UploadAttachmentBody>,
) -> impl IntoResponse {
    let bytes = match decode_base64(&body.data_base64) {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match store.save_attachment(body.folder.as_deref(), &body.filename, &bytes) {
        Ok(rel) => Json(serde_json::json!({ "name": rel })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn api_get_attachment(
    State(store): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match store.read_attachment(&name) {
        Ok(bytes) => {
            let mime = mime_from_name(&name);
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, mime)],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Attachment not found").into_response(),
    }
}

fn mime_from_name(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else {
        "application/octet-stream"
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    fn val(b: u8) -> Result<u32, String> {
        Ok(match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a' + 26) as u32,
            b'0'..=b'9' => (b - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("Invalid base64 character".into()),
        })
    }
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !cleaned.len().is_multiple_of(4) {
        return Err("Invalid base64 length".into());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        let c = if pad2 { 0 } else { val(chunk[2])? };
        let d = if pad3 { 0 } else { val(chunk[3])? };
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push((n >> 16) as u8);
        if !pad2 {
            out.push((n >> 8) as u8);
        }
        if !pad3 {
            out.push(n as u8);
        }
    }
    Ok(out)
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
