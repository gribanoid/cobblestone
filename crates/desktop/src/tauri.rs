// SPDX-License-Identifier: GPL-3.0-or-later
//
// Cobblestone — open-source knowledge base for your private thoughts
// Copyright (C) 2026  Cobblestone Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Thin async wrappers around `window.__TAURI__.core.invoke`.
//!
//! Each function serialises its arguments, calls the Tauri command by name,
//! and deserialises the result.  Any error is surfaced as a String so that
//! Leptos components can display it directly.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Low-level JS binding
// ---------------------------------------------------------------------------

#[wasm_bindgen]
extern "C" {
    /// Calls a registered Tauri command.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// ---------------------------------------------------------------------------
// Shared types (mirror of cobblestone-core::Note)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteInfo {
    pub slug:     String,
    pub title:    String,
    pub modified: String,
    pub size:     u64,
    pub preview:  String,
    pub tags:     Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteContent {
    pub slug:    String,
    pub title:   String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn js_err(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| "unknown error".into())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub async fn list_notes() -> Result<Vec<NoteInfo>, String> {
    let result = invoke("list_notes", JsValue::NULL)
        .await
        .map_err(js_err)?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

pub async fn get_note(slug: &str) -> Result<NoteContent, String> {
    #[derive(Serialize)]
    struct Args<'a> { slug: &'a str }
    let args = serde_wasm_bindgen::to_value(&Args { slug })
        .map_err(|e| e.to_string())?;
    let result = invoke("get_note", args).await.map_err(js_err)?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

pub async fn save_note(slug: &str, content: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> { slug: &'a str, content: &'a str }
    let args = serde_wasm_bindgen::to_value(&Args { slug, content })
        .map_err(|e| e.to_string())?;
    invoke("save_note", args).await.map_err(js_err)?;
    Ok(())
}

pub async fn create_note(title: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> { title: &'a str }
    let args = serde_wasm_bindgen::to_value(&Args { title })
        .map_err(|e| e.to_string())?;
    let result = invoke("create_note", args).await.map_err(js_err)?;
    result.as_string().ok_or_else(|| "expected string slug".into())
}

pub async fn delete_note(slug: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> { slug: &'a str }
    let args = serde_wasm_bindgen::to_value(&Args { slug })
        .map_err(|e| e.to_string())?;
    invoke("delete_note", args).await.map_err(js_err)?;
    Ok(())
}

pub async fn search_notes(query: &str) -> Result<Vec<NoteInfo>, String> {
    #[derive(Serialize)]
    struct Args<'a> { query: &'a str }
    let args = serde_wasm_bindgen::to_value(&Args { query })
        .map_err(|e| e.to_string())?;
    let result = invoke("search_notes", args).await.map_err(js_err)?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}
