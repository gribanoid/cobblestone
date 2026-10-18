// SPDX-License-Identifier: MIT

use async_trait::async_trait;
use gloo_net::http::Request;
use serde::Deserialize;

use super::VaultBackend;
use crate::models::{NoteContent, NoteGraph, NoteInfo, VaultNode};

#[derive(Clone, Default)]
pub struct HttpApi;

impl HttpApi {
    pub fn shared() -> super::SharedBackend {
        std::sync::Arc::new(Self)
    }
}

async fn api_json<T: for<'de> Deserialize<'de>>(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    let text = api_text(method, path, body).await?;
    if text.is_empty() {
        return Err("Empty response".into());
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

async fn api_empty(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<(), String> {
    let _ = api_text(method, path, body).await?;
    Ok(())
}

async fn api_text(
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<String, String> {
    let builder = match method {
        "GET" => Request::get(path),
        "POST" => Request::post(path),
        "PUT" => Request::put(path),
        "DELETE" => Request::delete(path),
        _ => return Err(format!("Unsupported method {method}")),
    };

    let resp = if let Some(b) = body {
        builder
            .header("Content-Type", "application/json")
            .json(&b)
            .map_err(|e| e.to_string())?
            .send()
            .await
    } else {
        builder.send().await
    }
    .map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(if text.is_empty() {
            format!("HTTP {}", resp.status())
        } else {
            text
        });
    }
    Ok(text)
}

#[derive(Deserialize)]
struct SlugResp {
    slug: String,
}

#[derive(Deserialize)]
struct PathResp {
    path: String,
}

#[async_trait(?Send)]
impl VaultBackend for HttpApi {
    async fn list_tree(&self) -> Result<Vec<VaultNode>, String> {
        api_json("GET", "/api/tree", None).await
    }

    async fn get_note(&self, slug: &str) -> Result<NoteContent, String> {
        let path = format!("/api/notes/{}", urlencoding(slug));
        api_json("GET", &path, None).await
    }

    async fn save_note(&self, slug: &str, content: &str) -> Result<(), String> {
        let path = format!("/api/notes/{}", urlencoding(slug));
        api_empty(
            "POST",
            &path,
            Some(serde_json::json!({ "content": content })),
        )
        .await
    }

    async fn create_note(&self, title: &str, folder: Option<&str>) -> Result<String, String> {
        let resp: SlugResp = api_json(
            "PUT",
            "/api/notes",
            Some(serde_json::json!({ "title": title, "folder": folder })),
        )
        .await?;
        Ok(resp.slug)
    }

    async fn create_folder(&self, path: &str) -> Result<(), String> {
        api_empty(
            "POST",
            "/api/folders",
            Some(serde_json::json!({ "path": path })),
        )
        .await
    }

    async fn move_note(&self, slug: &str, folder: Option<&str>) -> Result<String, String> {
        let resp: SlugResp = api_json(
            "POST",
            "/api/notes/move",
            Some(serde_json::json!({ "slug": slug, "folder": folder })),
        )
        .await?;
        Ok(resp.slug)
    }

    async fn move_folder(&self, path: &str, dest_parent: Option<&str>) -> Result<String, String> {
        let resp: PathResp = api_json(
            "POST",
            "/api/folders/move",
            Some(serde_json::json!({ "path": path, "dest_parent": dest_parent })),
        )
        .await?;
        Ok(resp.path)
    }

    async fn delete_note(&self, slug: &str) -> Result<(), String> {
        let path = format!("/api/notes/{}", urlencoding(slug));
        api_empty("DELETE", &path, None).await
    }

    async fn rename_note(&self, slug: &str, title: &str) -> Result<String, String> {
        let resp: SlugResp = api_json(
            "POST",
            "/api/notes/rename",
            Some(serde_json::json!({ "slug": slug, "title": title })),
        )
        .await?;
        Ok(resp.slug)
    }

    async fn rename_folder(&self, path: &str, name: &str) -> Result<String, String> {
        let resp: PathResp = api_json(
            "POST",
            "/api/folders/rename",
            Some(serde_json::json!({ "path": path, "name": name })),
        )
        .await?;
        Ok(resp.path)
    }

    async fn delete_folder(&self, path: &str) -> Result<(), String> {
        api_empty(
            "POST",
            "/api/folders/delete",
            Some(serde_json::json!({ "path": path })),
        )
        .await
    }

    async fn search_notes(&self, query: &str) -> Result<Vec<NoteInfo>, String> {
        let path = format!("/api/search?query={}", urlencoding(query));
        api_json("GET", &path, None).await
    }

    async fn note_graph(&self, slug: &str) -> Result<NoteGraph, String> {
        let path = format!("/api/notes/{}/graph", urlencoding(slug));
        api_json("GET", &path, None).await
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b'/' => out.push_str("%2F"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
