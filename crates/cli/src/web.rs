// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};

use cobblestone_core::Store;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

type AppState = Arc<Store>;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn run(store: Store, port: u16) -> Result<()> {
    let state = Arc::new(store);

    let app = Router::new()
        .route("/",                    get(handler_index))
        .route("/api/notes",           get(api_list))
        .route("/api/notes",           put(api_create))
        .route("/api/notes/{slug}",     get(api_get))
        .route("/api/notes/{slug}",     post(api_update))
        .route("/api/notes/{slug}",     delete(api_delete))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    let url  = format!("http://{addr}");
    println!("Cobblestone web  →  {url}");
    println!("Press Ctrl+C to stop.");

    // Try to open in browser
    let _ = open::that(&url);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handler_index() -> Html<&'static str> {
    Html(WEB_UI)
}

#[derive(Serialize)]
struct NoteInfo {
    slug:     String,
    title:    String,
    modified: String,
    size:     u64,
    preview:  String,
    tags:     Vec<String>,
}

async fn api_list(State(store): State<AppState>) -> impl IntoResponse {
    match store.list_notes() {
        Ok(notes) => {
            let list: Vec<NoteInfo> = notes
                .iter()
                .map(|n| NoteInfo {
                    slug:     n.name.clone(),
                    title:    n.title.clone(),
                    modified: n.modified.clone(),
                    size:     n.size,
                    preview:  n.preview.clone(),
                    tags:     n.tags.clone(),
                })
                .collect();
            (StatusCode::OK, Json(list)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Serialize)]
struct NoteContent {
    slug:    String,
    title:   String,
    content: String,
}

async fn api_get(
    State(store): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match store.read(&slug) {
        Ok(content) => {
            let title = content
                .lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").to_string())
                .unwrap_or_else(|| slug.clone());
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
        Ok(_)  => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateBody {
    title: String,
}

async fn api_create(
    State(store): State<AppState>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    let slug = cobblestone_core::slugify(&body.title);
    if store.exists(&slug) {
        return (StatusCode::CONFLICT, "Note already exists").into_response();
    }
    let date    = chrono::Local::now().format("%Y-%m-%d").to_string();
    let content = format!("# {}\n\n*Created: {}*\n\n", body.title, date);
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
        Ok(_)  => StatusCode::OK.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Note not found").into_response(),
    }
}

// ---------------------------------------------------------------------------
// Embedded Web UI
// ---------------------------------------------------------------------------

const WEB_UI: &str = r#"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Cobblestone</title>
<style>
/* ─── Themes ──────────────────────────────────────────────────────────────── */
:root {
  --bg:        #1e1e2e;
  --surface:   #181825;
  --surface2:  #313244;
  --overlay:   #45475a;
  --accent:    #cba6f7;
  --accent2:   #89b4fa;
  --green:     #a6e3a1;
  --red:       #f38ba8;
  --yellow:    #f9e2af;
  --peach:     #fab387;
  --text:      #cdd6f4;
  --subtext:   #bac2de;
  --muted:     #6c7086;
  --border:    #313244;
  --sidebar-w: 280px;
}
[data-theme="light"] {
  --bg:       #eff1f5;
  --surface:  #e6e9ef;
  --surface2: #dce0e8;
  --overlay:  #bcc0cc;
  --accent:   #8839ef;
  --accent2:  #1e66f5;
  --green:    #40a02b;
  --red:      #d20f39;
  --yellow:   #df8e1d;
  --peach:    #fe640b;
  --text:     #4c4f69;
  --subtext:  #5c5f77;
  --muted:    #9ca0b0;
  --border:   #ccd0da;
}

/* ─── Reset ────────────────────────────────────────────────────────────────── */
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
html,body{height:100%;font-size:15px}
body{
  font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;
  background:var(--bg);color:var(--text);height:100vh;
  display:flex;flex-direction:column;overflow:hidden;
}
button{cursor:pointer;border:none;outline:none;font:inherit}
input{font:inherit;outline:none}
textarea{font:inherit;outline:none}
::-webkit-scrollbar{width:6px;height:6px}
::-webkit-scrollbar-track{background:transparent}
::-webkit-scrollbar-thumb{background:var(--surface2);border-radius:3px}

/* ─── Layout ──────────────────────────────────────────────────────────────── */
.app{display:flex;height:100vh;overflow:hidden}

/* ─── Sidebar ─────────────────────────────────────────────────────────────── */
.sidebar{
  width:var(--sidebar-w);min-width:var(--sidebar-w);
  background:var(--surface);
  border-right:1px solid var(--border);
  display:flex;flex-direction:column;
  overflow:hidden;
  transition:width 0.2s;
}
.sidebar-header{
  display:flex;align-items:center;justify-content:space-between;
  padding:16px 14px 10px;
  border-bottom:1px solid var(--border);
}
.logo{
  font-weight:700;font-size:16px;
  color:var(--accent);letter-spacing:0.3px;
  display:flex;align-items:center;gap:6px;
}
.header-btns{display:flex;gap:6px}
.icon-btn{
  background:none;color:var(--muted);
  padding:4px 6px;border-radius:6px;font-size:14px;
  transition:color 0.15s,background 0.15s;
}
.icon-btn:hover{background:var(--surface2);color:var(--text)}

.search-wrap{padding:10px 12px 6px}
.search-input{
  width:100%;background:var(--surface2);
  border:1px solid var(--border);border-radius:8px;
  color:var(--text);padding:7px 12px;font-size:13px;
  transition:border-color 0.15s;
}
.search-input:focus{border-color:var(--accent)}
.search-input::placeholder{color:var(--muted)}

.new-btn{
  margin:4px 12px 10px;
  background:var(--accent);color:var(--bg);
  border-radius:8px;padding:7px 14px;font-size:13px;
  font-weight:600;transition:opacity 0.15s;
}
.new-btn:hover{opacity:0.85}

.note-list{flex:1;overflow-y:auto;padding:2px 0}
.note-item{
  padding:10px 14px;cursor:pointer;
  border-left:3px solid transparent;
  transition:background 0.1s,border-color 0.1s;
}
.note-item:hover{background:var(--surface2)}
.note-item.active{
  background:var(--surface2);
  border-left-color:var(--accent);
}
.note-item-title{
  font-size:13px;font-weight:600;color:var(--text);
  white-space:nowrap;overflow:hidden;text-overflow:ellipsis;
}
.note-item-meta{
  font-size:11px;color:var(--muted);margin-top:2px;
  white-space:nowrap;overflow:hidden;text-overflow:ellipsis;
}
.note-item-tags{margin-top:3px;display:flex;gap:4px;flex-wrap:wrap}
.tag{
  font-size:10px;padding:1px 6px;border-radius:10px;
  background:var(--surface);color:var(--accent);border:1px solid var(--border);
}
.empty-state{
  padding:32px 16px;text-align:center;color:var(--muted);font-size:13px;line-height:1.8;
}

/* ─── Main Content ────────────────────────────────────────────────────────── */
.content{flex:1;display:flex;flex-direction:column;min-width:0;overflow:hidden}

.toolbar{
  display:flex;align-items:center;justify-content:space-between;
  padding:10px 20px;
  border-bottom:1px solid var(--border);
  background:var(--surface);
  gap:12px;
}
.note-title-input{
  flex:1;background:none;border:none;
  color:var(--text);font-size:16px;font-weight:600;
  min-width:0;
}
.note-title-input::placeholder{color:var(--muted)}
.toolbar-actions{display:flex;gap:8px;align-items:center}
.tb-btn{
  padding:5px 14px;border-radius:7px;font-size:13px;font-weight:500;
  background:var(--surface2);color:var(--subtext);
  transition:background 0.15s,color 0.15s;
}
.tb-btn:hover{background:var(--overlay);color:var(--text)}
.tb-btn.primary{background:var(--accent);color:var(--bg)}
.tb-btn.primary:hover{opacity:0.85}
.tb-btn.danger{background:none;color:var(--red)}
.tb-btn.danger:hover{background:var(--red);color:var(--bg)}
.tb-btn.active{background:var(--accent2);color:var(--bg)}
.edit-indicator{
  font-size:11px;color:var(--muted);
  padding:2px 8px;border-radius:10px;background:var(--surface2);
}
.edit-indicator.unsaved{color:var(--peach)}

.editor-wrap{flex:1;display:flex;min-height:0;position:relative}
#editor{
  flex:1;width:100%;height:100%;resize:none;
  background:var(--bg);color:var(--text);
  border:none;padding:24px 28px;
  font-family:'JetBrains Mono','Fira Code','Cascadia Code',monospace;
  font-size:14px;line-height:1.7;
  tab-size:2;
}
#editor.hidden{display:none}

.preview{
  flex:1;overflow-y:auto;padding:24px 40px;
  font-size:15px;line-height:1.8;
  max-width:800px;margin:0 auto;width:100%;
}
.preview.hidden{display:none}

/* ─── Preview Typography ──────────────────────────────────────────────────── */
.preview h1{font-size:2em;color:var(--yellow);border-bottom:1px solid var(--border);padding-bottom:8px;margin:0 0 16px}
.preview h2{font-size:1.4em;color:var(--accent2);margin:24px 0 12px}
.preview h3{font-size:1.15em;color:var(--green);margin:20px 0 10px}
.preview h4,.preview h5,.preview h6{color:var(--peach);margin:16px 0 8px}
.preview p{margin:0 0 12px;color:var(--text)}
.preview a{color:var(--accent2);text-decoration:none}
.preview a:hover{text-decoration:underline}
.preview strong{color:var(--text);font-weight:700}
.preview em{color:var(--subtext)}
.preview del{color:var(--muted)}
.preview hr{border:none;border-top:1px solid var(--border);margin:24px 0}
.preview blockquote{
  border-left:3px solid var(--accent);
  padding:6px 16px;margin:12px 0;
  color:var(--subtext);background:var(--surface);
  border-radius:0 6px 6px 0;
}
.preview code{
  font-family:'JetBrains Mono','Fira Code',monospace;
  font-size:0.85em;padding:2px 6px;border-radius:4px;
  background:var(--surface2);color:var(--peach);
}
.preview pre{
  background:var(--surface);border:1px solid var(--border);
  border-radius:8px;padding:16px;overflow-x:auto;margin:12px 0;
}
.preview pre code{background:none;color:var(--green);padding:0;font-size:13px;line-height:1.6}
.preview ul,.preview ol{padding-left:24px;margin:8px 0 12px}
.preview li{margin:4px 0;color:var(--text)}
.preview table{border-collapse:collapse;width:100%;margin:16px 0}
.preview th,.preview td{border:1px solid var(--border);padding:8px 12px;text-align:left}
.preview th{background:var(--surface);font-weight:600;color:var(--subtext)}
/* Checkboxes */
.preview .task-item{list-style:none;margin-left:-24px;padding-left:4px}
.preview .task-item input[type=checkbox]{
  margin-right:8px;accent-color:var(--green);width:14px;height:14px;vertical-align:middle;
}
.preview .task-item.done{color:var(--muted);text-decoration:line-through}

/* ─── Welcome screen ─────────────────────────────────────────────────────── */
.welcome{
  flex:1;display:flex;flex-direction:column;
  align-items:center;justify-content:center;
  color:var(--muted);text-align:center;gap:12px;
  padding:40px;
}
.welcome .logo-big{font-size:48px}
.welcome h2{font-size:22px;color:var(--text)}
.welcome p{font-size:14px;max-width:360px;line-height:1.7}
.welcome kbd{
  font-family:monospace;font-size:12px;
  background:var(--surface2);border:1px solid var(--border);
  border-radius:4px;padding:2px 7px;color:var(--subtext);
}

/* ─── Toast ──────────────────────────────────────────────────────────────── */
#toast{
  position:fixed;bottom:24px;right:24px;
  background:var(--surface2);color:var(--text);
  padding:10px 18px;border-radius:8px;font-size:13px;
  opacity:0;pointer-events:none;
  transition:opacity 0.2s;
  border:1px solid var(--border);
}
#toast.show{opacity:1}

/* ─── Modal ───────────────────────────────────────────────────────────────── */
.modal-overlay{
  position:fixed;inset:0;background:rgba(0,0,0,0.5);
  display:flex;align-items:center;justify-content:center;
  z-index:100;
}
.modal-overlay.hidden{display:none}
.modal{
  background:var(--surface);border:1px solid var(--border);
  border-radius:12px;padding:28px;min-width:340px;max-width:480px;
}
.modal h3{margin-bottom:16px;color:var(--text)}
.modal input{
  width:100%;background:var(--surface2);border:1px solid var(--border);
  color:var(--text);padding:9px 14px;border-radius:8px;font-size:14px;
}
.modal input:focus{border-color:var(--accent)}
.modal-actions{display:flex;gap:10px;justify-content:flex-end;margin-top:20px}
</style>
</head>
<body>

<div class="app">
  <!-- ── Sidebar ─────────────────────────────────────────────────── -->
  <nav class="sidebar">
    <div class="sidebar-header">
      <span class="logo">🪨 Cobblestone</span>
      <div class="header-btns">
        <button class="icon-btn" onclick="toggleTheme()" title="Toggle theme">◑</button>
        <button class="icon-btn" onclick="showShortcuts()" title="Shortcuts">?</button>
      </div>
    </div>
    <div class="search-wrap">
      <input type="text" class="search-input" id="searchInput"
             placeholder="Search notes…" oninput="filterNotes(this.value)">
    </div>
    <button class="new-btn" onclick="openNewModal()">+ New note</button>
    <div class="note-list" id="noteList">
      <div class="empty-state">Loading…</div>
    </div>
  </nav>

  <!-- ── Main ───────────────────────────────────────────────────── -->
  <main class="content" id="mainContent">
    <div class="welcome" id="welcomeScreen">
      <div class="logo-big">🪨</div>
      <h2>Welcome to Cobblestone</h2>
      <p>Write anything. Store everything. Own it all.<br>
         Select a note to read it, or create a new one.</p>
      <p><kbd>n</kbd> new &nbsp; <kbd>e</kbd> edit &nbsp;
         <kbd>/</kbd> search &nbsp; <kbd>Ctrl+S</kbd> save</p>
    </div>

    <div id="noteArea" class="hidden" style="display:none;flex-direction:column;flex:1;min-height:0">
      <div class="toolbar">
        <input class="note-title-input" id="noteTitleInput"
               placeholder="Note title" oninput="markDirty()">
        <div class="toolbar-actions">
          <span class="edit-indicator" id="saveIndicator">saved</span>
          <button class="tb-btn active" id="viewToggle" onclick="toggleMode()">Preview</button>
          <button class="tb-btn primary" onclick="saveNote()">Save</button>
          <button class="tb-btn danger" onclick="deleteNote()">Delete</button>
        </div>
      </div>
      <div class="editor-wrap">
        <textarea id="editor" spellcheck="true"
                  placeholder="Start writing in Markdown…"
                  oninput="markDirty(); updateWordCount()"></textarea>
        <div id="preview" class="preview hidden"></div>
      </div>
    </div>
  </main>
</div>

<!-- ── New note modal ─────────────────────────────────────────────────── -->
<div class="modal-overlay hidden" id="newModal">
  <div class="modal">
    <h3>New note</h3>
    <input type="text" id="newNoteTitle" placeholder="Note title…"
           onkeydown="if(event.key==='Enter')confirmNew();if(event.key==='Escape')closeNewModal()">
    <div class="modal-actions">
      <button class="tb-btn" onclick="closeNewModal()">Cancel</button>
      <button class="tb-btn primary" onclick="confirmNew()">Create</button>
    </div>
  </div>
</div>

<!-- ── Toast ──────────────────────────────────────────────────────────── -->
<div id="toast"></div>

<script>
// ────────────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────────────
let notes       = [];
let activeSlug  = null;
let editing     = false;   // false = preview, true = editor
let dirty       = false;
let saveTimer   = null;

// ────────────────────────────────────────────────────────────────────────────
// Boot
// ────────────────────────────────────────────────────────────────────────────
async function init() {
  const theme = localStorage.getItem('cb-theme') || 'dark';
  document.documentElement.dataset.theme = theme;
  await loadNoteList();
  setupKeyboard();
}

// ────────────────────────────────────────────────────────────────────────────
// API
// ────────────────────────────────────────────────────────────────────────────
async function api(method, path, body) {
  const opts = { method, headers: { 'Content-Type': 'application/json' } };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const r = await fetch(path, opts);
  if (!r.ok) throw new Error(await r.text());
  if (r.status === 200 && r.headers.get('content-type')?.includes('json'))
    return r.json();
}

// ────────────────────────────────────────────────────────────────────────────
// Note list
// ────────────────────────────────────────────────────────────────────────────
async function loadNoteList() {
  notes = await api('GET', '/api/notes') || [];
  renderNoteList(notes);
}

function renderNoteList(list) {
  const el = document.getElementById('noteList');
  if (list.length === 0) {
    el.innerHTML = '<div class="empty-state">No notes yet.<br>Click "+ New note" to start.</div>';
    return;
  }
  el.innerHTML = list.map(n => `
    <div class="note-item ${n.slug === activeSlug ? 'active' : ''}"
         data-slug="${esc(n.slug)}">
      <div class="note-item-title">${esc(n.title)}</div>
      <div class="note-item-meta">${esc(n.modified)}${n.size ? ' · ' + humanSize(n.size) : ''}</div>
      ${n.tags.length ? `<div class="note-item-tags">${n.tags.map(t => `<span class="tag">#${esc(t)}</span>`).join('')}</div>` : ''}
    </div>
  `).join('');
  // Attach click handlers via addEventListener (never inline onclick) to avoid XSS
  el.querySelectorAll('.note-item[data-slug]').forEach(el => {
    el.addEventListener('click', () => openNote(el.dataset.slug));
  });
}

function filterNotes(q) {
  const ql = q.toLowerCase();
  const filtered = ql
    ? notes.filter(n => n.title.toLowerCase().includes(ql) || n.preview.toLowerCase().includes(ql))
    : notes;
  renderNoteList(filtered);
}

// ────────────────────────────────────────────────────────────────────────────
// Open / edit note
// ────────────────────────────────────────────────────────────────────────────
async function openNote(slug) {
  if (dirty && activeSlug) {
    if (!confirm('You have unsaved changes. Discard?')) return;
  }
  const data = await api('GET', `/api/notes/${slug}`);
  activeSlug = slug;
  dirty = false;
  editing = false;

  document.getElementById('welcomeScreen').style.display = 'none';
  const area = document.getElementById('noteArea');
  area.style.display = 'flex';

  document.getElementById('noteTitleInput').value = data.title;
  document.getElementById('editor').value = data.content;
  document.getElementById('saveIndicator').textContent = 'saved';
  document.getElementById('saveIndicator').className = 'edit-indicator';

  setMode('preview');
  renderPreview(data.content);
  updateActiveItem(slug);
}

function setMode(mode) {
  const editorEl  = document.getElementById('editor');
  const previewEl = document.getElementById('preview');
  const toggleBtn = document.getElementById('viewToggle');

  if (mode === 'editor') {
    editing = true;
    editorEl.classList.remove('hidden');
    previewEl.classList.add('hidden');
    toggleBtn.textContent = 'Preview';
    toggleBtn.className = 'tb-btn';
    editorEl.focus();
  } else {
    editing = false;
    editorEl.classList.add('hidden');
    previewEl.classList.remove('hidden');
    toggleBtn.textContent = 'Edit';
    toggleBtn.className = 'tb-btn active';
    renderPreview(editorEl.value);
  }
}

function toggleMode() {
  setMode(editing ? 'preview' : 'editor');
}

function markDirty() {
  if (!dirty) {
    dirty = true;
    document.getElementById('saveIndicator').textContent = '● unsaved';
    document.getElementById('saveIndicator').className = 'edit-indicator unsaved';
  }
  // auto-save after 2 s idle
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNote, 2000);
}

async function saveNote() {
  if (!activeSlug) return;
  const content = document.getElementById('editor').value;
  const title   = document.getElementById('noteTitleInput').value.trim();

  // Update title in content if first line is h1
  let final = content;
  if (title) {
    const lines = content.split('\n');
    if (lines[0].startsWith('# ')) {
      lines[0] = `# ${title}`;
      final = lines.join('\n');
      document.getElementById('editor').value = final;
    }
  }

  try {
    await api('POST', `/api/notes/${activeSlug}`, { content: final });
    dirty = false;
    document.getElementById('saveIndicator').textContent = 'saved';
    document.getElementById('saveIndicator').className = 'edit-indicator';
    toast('Saved ✓');
    await loadNoteList();
  } catch (e) {
    toast('Save failed: ' + e.message, true);
  }
}

async function deleteNote() {
  if (!activeSlug) return;
  if (!confirm(`Delete this note? This cannot be undone.`)) return;
  await api('DELETE', `/api/notes/${activeSlug}`);
  activeSlug = null;
  dirty = false;
  document.getElementById('noteArea').style.display = 'none';
  document.getElementById('welcomeScreen').style.display = '';
  await loadNoteList();
  toast('Deleted');
}

// ────────────────────────────────────────────────────────────────────────────
// New note modal
// ────────────────────────────────────────────────────────────────────────────
function openNewModal() {
  const modal = document.getElementById('newModal');
  modal.classList.remove('hidden');
  document.getElementById('newNoteTitle').value = '';
  document.getElementById('newNoteTitle').focus();
}
function closeNewModal() {
  document.getElementById('newModal').classList.add('hidden');
}
async function confirmNew() {
  const title = document.getElementById('newNoteTitle').value.trim();
  if (!title) return;
  closeNewModal();
  try {
    const { slug } = await api('PUT', '/api/notes', { title });
    await loadNoteList();
    await openNote(slug);
    setMode('editor');
    toast('Note created');
  } catch (e) {
    toast('Error: ' + e.message, true);
  }
}

// ────────────────────────────────────────────────────────────────────────────
// Markdown renderer
// ────────────────────────────────────────────────────────────────────────────
function renderPreview(md) {
  document.getElementById('preview').innerHTML = parseMarkdown(md);
  // make checkboxes non-interactive (read-only display in preview)
  document.querySelectorAll('.preview input[type=checkbox]').forEach(cb => {
    cb.addEventListener('change', e => { e.preventDefault(); cb.checked = !cb.checked; });
  });
}

function parseMarkdown(md) {
  const lines = md.split('\n');
  let html = '';
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    if (line.startsWith('```')) {
      const lang = line.slice(3).trim();
      let code = '';
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) {
        code += escHtml(lines[i]) + '\n';
        i++;
      }
      html += `<pre><code class="lang-${esc(lang)}">${code}</code></pre>\n`;
      i++;
      continue;
    }

    // Headings
    const hm = line.match(/^(#{1,6})\s+(.*)/);
    if (hm) {
      const lvl = hm[1].length;
      html += `<h${lvl}>${inlineMd(hm[2])}</h${lvl}>\n`;
      i++; continue;
    }

    // Horizontal rule
    if (/^[-*_]{3,}\s*$/.test(line)) { html += '<hr>\n'; i++; continue; }

    // Blockquote
    if (line.startsWith('> ')) {
      let content = '';
      while (i < lines.length && lines[i].startsWith('> ')) {
        content += lines[i].slice(2) + '\n';
        i++;
      }
      html += `<blockquote>${parseMarkdown(content.trim())}</blockquote>\n`;
      continue;
    }

    // Unordered list (including task list)
    if (/^[-*+] /.test(line)) {
      html += '<ul>';
      while (i < lines.length && /^[-*+] /.test(lines[i])) {
        const t = lines[i].slice(2);
        if (t.startsWith('[ ] ')) {
          html += `<li class="task-item"><input type="checkbox"> ${inlineMd(t.slice(4))}</li>`;
        } else if (t.startsWith('[x] ') || t.startsWith('[X] ')) {
          html += `<li class="task-item done"><input type="checkbox" checked> ${inlineMd(t.slice(4))}</li>`;
        } else {
          html += `<li>${inlineMd(t)}</li>`;
        }
        i++;
      }
      html += '</ul>\n';
      continue;
    }

    // Ordered list
    if (/^\d+\. /.test(line)) {
      html += '<ol>';
      while (i < lines.length && /^\d+\. /.test(lines[i])) {
        html += `<li>${inlineMd(lines[i].replace(/^\d+\. /, ''))}</li>`;
        i++;
      }
      html += '</ol>\n';
      continue;
    }

    // Table
    if (line.includes('|') && i + 1 < lines.length && lines[i+1].match(/^\|?[-| :]+\|?$/)) {
      const headers = line.split('|').map(s => s.trim()).filter(Boolean);
      html += '<table><thead><tr>' + headers.map(h => `<th>${inlineMd(h)}</th>`).join('') + '</tr></thead><tbody>';
      i += 2;
      while (i < lines.length && lines[i].includes('|')) {
        const cells = lines[i].split('|').map(s => s.trim()).filter(Boolean);
        html += '<tr>' + cells.map(c => `<td>${inlineMd(c)}</td>`).join('') + '</tr>';
        i++;
      }
      html += '</tbody></table>\n';
      continue;
    }

    // Empty line → paragraph break
    if (line.trim() === '') { html += '<p></p>\n'; i++; continue; }

    // Paragraph
    let para = '';
    while (i < lines.length && lines[i].trim() !== '' &&
           !lines[i].startsWith('#') && !lines[i].startsWith('>') &&
           !/^[-*+] /.test(lines[i]) && !/^\d+\. /.test(lines[i]) &&
           !lines[i].startsWith('```') && !/^[-*_]{3,}\s*$/.test(lines[i])) {
      para += lines[i] + ' ';
      i++;
    }
    if (para.trim()) html += `<p>${inlineMd(para.trim())}</p>\n`;
  }

  return html;
}

function inlineMd(s) {
  return s
    .replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
    .replace(/`([^`]+)`/g,      '<code>$1</code>')
    .replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img src="$2" alt="$1" style="max-width:100%">')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g,  '<a href="$2" target="_blank">$1</a>')
    .replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>')
    .replace(/\*\*(.+?)\*\*/g,    '<strong>$1</strong>')
    .replace(/__(.+?)__/g,        '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g,        '<em>$1</em>')
    .replace(/_(.+?)_/g,          '<em>$1</em>')
    .replace(/~~(.+?)~~/g,        '<del>$1</del>');
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

// ────────────────────────────────────────────────────────────────────────────
// Utilities
// ────────────────────────────────────────────────────────────────────────────
function esc(s) {
  return String(s)
    .replace(/&/g,'&amp;').replace(/</g,'&lt;')
    .replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function humanSize(b) {
  if (b < 1024)       return b + ' B';
  if (b < 1048576)    return (b/1024).toFixed(1) + ' KB';
  return (b/1048576).toFixed(1) + ' MB';
}

function updateActiveItem(slug) {
  document.querySelectorAll('.note-item').forEach(el => {
    el.classList.toggle('active', el.dataset.slug === slug);
  });
}

function updateWordCount() {
  const txt = document.getElementById('editor').value;
  const wc  = txt.trim().split(/\s+/).filter(Boolean).length;
}

function toast(msg, err = false) {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.style.borderColor = err ? 'var(--red)' : 'var(--border)';
  el.classList.add('show');
  setTimeout(() => el.classList.remove('show'), 2500);
}

function toggleTheme() {
  const d = document.documentElement;
  const t = d.dataset.theme === 'dark' ? 'light' : 'dark';
  d.dataset.theme = t;
  localStorage.setItem('cb-theme', t);
}

function showShortcuts() {
  toast('n: new  e: edit  Ctrl+S: save  /: search  Esc: cancel');
}

// ────────────────────────────────────────────────────────────────────────────
// Keyboard shortcuts
// ────────────────────────────────────────────────────────────────────────────
function setupKeyboard() {
  document.addEventListener('keydown', e => {
    const tag = document.activeElement.tagName;
    const inInput = tag === 'INPUT' || tag === 'TEXTAREA';

    if (e.key === 's' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      saveNote();
      return;
    }

    if (!inInput) {
      if (e.key === 'n') { e.preventDefault(); openNewModal(); }
      if (e.key === 'e' && activeSlug) { e.preventDefault(); setMode('editor'); }
      if (e.key === '/') { e.preventDefault(); document.getElementById('searchInput').focus(); }
      if (e.key === 'Escape') { document.getElementById('searchInput').value = ''; filterNotes(''); }
    }

    if (inInput && e.key === 'Escape') {
      document.activeElement.blur();
      if (!document.getElementById('newModal').classList.contains('hidden')) {
        closeNewModal();
      }
    }

    // Tab in editor → insert 2 spaces
    if (tag === 'TEXTAREA' && e.key === 'Tab') {
      e.preventDefault();
      const ta  = document.getElementById('editor');
      const s   = ta.selectionStart;
      ta.value  = ta.value.slice(0, s) + '  ' + ta.value.slice(ta.selectionEnd);
      ta.selectionStart = ta.selectionEnd = s + 2;
      markDirty();
    }
  });
}

init();
</script>
</body>
</html>"#;
