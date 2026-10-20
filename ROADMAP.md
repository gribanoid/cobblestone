# 🗺 Cobblestone Roadmap

Items are roughly ordered by priority within each milestone.

**Current focus:** v0.2 — polish and power-user features. See also [README](README.md) for what ships today.

---

## ✅ v0.1 — Foundation

- [x] Plain Markdown storage in `~/.cobblestone`
- [x] `cobblestone-core` shared library (workspace crate)
- [x] CLI (`cb`) — `ls`, `new`, `show`, `edit`, `rm`, `search`
- [x] `-i` flag — full-screen TUI (ratatui + crossterm)
- [x] `cb web` — embedded local web server (axum) with browser UI
- [x] Web UI — dark/light theme, live Markdown preview, auto-save
- [x] Desktop app scaffold (historically Tauri; now Dioxus)
- [x] Tag extraction (`#tag` syntax), full-text search
- [x] MIT license

---

## 🔧 v0.2 — Polish & Power User *(current)*

### Shipped

- [x] **Rust-only graphical UI** — shared Dioxus UI (`cobblestone-ui`) for desktop and web WASM
- [x] **Desktop app** — Dioxus desktop + in-process `cobblestone-core` (`DirectStore`)
- [x] **Desktop note CRUD** — list, open, create, edit, save, delete
- [x] **Desktop backend search** — sidebar uses the shared storage search path
- [x] **Desktop note metadata panel** — tags, size, modified date, links, backlinks
- [x] **Delete confirmation** — destructive actions ask before removing notes or folders
- [x] **Nested file tree** — hierarchical folders and notes in web & desktop
- [x] **File operations in graphical UI** — create, rename, move, delete notes and folders; drag-and-drop; context menus
- [x] **Nested storage in core** — subdirectories, `list_tree()`, move/rename/delete folder APIs
- [x] **Wikilink graph in core** — `extract_wikilinks()`, `Store::note_graph()` shared by web API and desktop
- [x] **Wikilinks info panel** — outgoing links and backlinks shown in the side panel (clickable)
- [x] **`cobblestone-editor`** — own near-WYSIWYG Markdown editor crate (replaces CodeMirror); live lists/headings, `![[image]]` embeds, paste-to-attachments
- [x] **EditorSession + scenario TDD** — pure `EditorCmd`/`EditorEffect` orchestration (`session.rs`); History snapshots restore focus/caret/reveal; regression scenarios for caret, lists, embeds, selection
- [x] **Editor block kinds** — task lists `- [ ]`, blockquotes `>`, horizontal rules; inline tokenizer for live-hide (`inline.rs`)

### Next up

- [ ] **Workspace folder selection** — open an existing Markdown folder instead of only `~/.cobblestone`
- [ ] **Wikilinks in preview** — click `[[Note Title]]` in the rendered Markdown body
- [ ] **Backlink line highlights** — highlight lines that mention the current note
- [ ] **Daily note** — `cb today` opens/creates a note for today's date
- [ ] **Note templates** — `cb new --template meeting`
- [ ] **Pinned notes** — appear at top of list
- [ ] **Configurable storage** — `CB_DIR` env var or `~/.config/cobblestone/config.toml`
- [ ] **Shell completions** — bash, zsh, fish via `clap_complete`
- [ ] **TUI folder tree** — hierarchical navigation in `cb -i` (today: flat list of all notes)
- [ ] **TUI inline editing** — edit without leaving `cb -i`
- [ ] **Mouse support** in TUI

---

## 🖥 v0.3 — Desktop App Full Feature

- [x] **Right information panel** — backlinks, tags, metadata, and outgoing links *(shipped in v0.2)*
- [ ] **Native file watcher** — reload note list when files change on disk
- [ ] **Graph view** — force-directed link graph between notes
- [ ] **Split editor/preview** — side-by-side in desktop window
- [ ] **Drag-and-drop attachments** — images stored in `~/.cobblestone/assets/`
- [ ] **Native menu bar** — File / Edit / View menus
- [ ] **System tray** — quick access to create note from anywhere
- [ ] **Native notifications** — remind to open daily note
- [ ] **Custom CSS themes** — drop `~/.cobblestone/theme.css` to restyle

---

## 🧱 Architecture Blueprint

Product constraints for the desktop app (longer-term):

| Area | Status |
|------|--------|
| **Local folder as workspace** | Planned — vault path still defaults to `~/.cobblestone` |
| **File operations** | Done — CRUD for notes and folders in core + graphical UI |
| **External change detection** | Planned |
| **Markdown-first editor** | Done (v0) — `cobblestone-editor` + `EditorSession` TDD; tasks/quotes/HR; inline tokenizer; further polish in v0.4 |
| **Connected notes** | Partial — wikilink parse, graph, backlinks panel; no autocomplete, broken-link hints, or preview navigation |
| **Search and navigation** | Partial — full-text search; no quick switcher, filters, or bookmarks |
| **Organization** | Partial — tags; no favorites, daily notes, templates, or global tag list |
| **Knowledge graph** | Planned — local note graph exists; no global graph view yet |
| **Portability** | Planned — import/export, HTML/PDF |
| **Settings** | Planned — workspace path, shortcuts, theme, templates |

---

## 🌐 v0.4 — Web UI Enhancements

- [ ] **Polish `cobblestone-editor`** — wire idle-line emphasis live-hide via `inline_display_text`, better mobile caret; keep expanding `session` scenarios (see `session::scenarios`)
- [x] **EditorSession undo/redo + block selection** — text + caret snapshots; cross-block delete/cut via `EditorCmd` *(shipped with TDD harness)*
- [ ] **Search match highlights** — highlight matching lines in search results
- [x] **Folder/category support** — subdirectories in storage *(shipped in v0.2)*
- [ ] **PWA manifest** — installable as desktop app from browser
- [ ] **Offline-first service worker**

*(Wikilink navigation in preview moved to v0.2 — shared UI work. CodeMirror dropped in favour of `cobblestone-editor`.)*

---

## ✏️ Editor — longer term

Markdown near-WYSIWYG ships first in `cobblestone-editor`. After that mode is stable:

- [ ] **Universal editor** — evolve the same crate beyond Markdown: plain text / code fences with language-aware highlighting, optional gutter/line numbers, shared keymap layer (without bringing back CodeMirror/Monaco)
- [ ] **Vim key-bindings** — optional mode on top of the universal editor *(also listed under Ideas)*

---

## 🔔 v0.5 — Notifications & Time-based Events

- [ ] **Due dates** — add `due: 2026-06-01` to any note's front-matter; `cb` picks it up automatically
- [ ] **Reminders** — `cb remind <slug> <time>` schedules a one-shot system notification (`notify-send` / `osascript` / Windows toast)
- [ ] **Recurring events** — cron-like syntax in front-matter: `repeat: every Monday 09:00`
- [ ] **Overdue warnings** — `cb` and TUI highlight notes with passed due dates in red
- [ ] **Agenda view** — `cb agenda` shows all notes with upcoming due dates sorted by time
- [ ] **Event daemon** — optional background process (`cb daemon`) that watches for due events and fires notifications without the app being open
- [ ] **Calendar export** — `cb export --ical` generates an `.ics` file compatible with any calendar app
- [ ] **Native desktop notifications** — OS-level alerts (macOS Notification Center, Linux libnotify, Windows toast)
- [ ] **Web UI agenda widget** — sidebar panel listing upcoming deadlines

---

## 🔁 v0.6 — Sync & Export

- [ ] **Git integration** — `cb sync` auto-commits and pushes
- [ ] **Import from Markdown folder** — point at any directory
- [ ] **Single note export** — `cb export <slug> --format html|pdf|md|txt`
- [ ] **Full vault export** — export all notes and assets as a portable folder or `.zip` archive
- [ ] **Website export** — generate a static HTML site from selected notes or the whole vault
- [ ] **JSON export** — machine-readable backup with notes, tags, metadata, and links
- [ ] **Encrypted notes** — per-note opt-in (age / GPG)
- [ ] **Conflict resolution** — smart merge for concurrent edits

---

## 🤖 v0.7 — Intelligence (Local)

- [ ] **Local LLM** — summarize / expand notes via [ollama](https://ollama.ai)
- [ ] **Smart tags** — auto-suggest tags based on content
- [ ] **Semantic search** — vector embeddings (local model, no API keys)
- [ ] **Spaced repetition** — flashcard mode for knowledge review

---

## 📦 Distribution

### Shipped

- [x] **GitHub Actions release pipeline** — tag `v*` builds `.dmg` (macOS Apple Silicon), `.deb` / `.AppImage` (Linux), `.msi` / `.exe` (Windows); draft [GitHub Releases](https://github.com/gribanoid/cobblestone/releases)
- [x] **Pre-built desktop binaries** — macOS arm64, Linux x86_64, Windows x86_64 (unsigned)

### Next up

- [ ] Homebrew formula (`brew install cobblestone`)
- [ ] **macOS code signing & notarization** — Apple Developer ID so Gatekeeper stops blocking downloads (see below)
- [ ] AUR package (Arch Linux)
- [ ] Flatpak (Linux)
- [ ] `cargo-binstall` support

### macOS: damaged and can't be opened

Release builds are **unsigned** (no Apple Developer Program yet). macOS Gatekeeper blocks them and shows a misleading *"damaged"* message — the app is fine.

**Workaround** (after installing from `.dmg`):

```bash
xattr -cr /Applications/Cobblestone.app
```

Or: right-click the app → **Open** → **Open** (first launch only).

**Permanent fix (planned):** sign and notarize the app in CI with an Apple Developer certificate.

---

## 💡 Ideas Backlog

- Mobile companion app (Dioxus mobile target — iOS/Android)
- Browser extension to clip web pages as notes
- CRDT-based local sync (Automerge)
- Vim key-bindings mode (via universal `cobblestone-editor`)
- YAML front-matter support (`---` header parsing)
- Note version history (per-note git blame)

---

Contributions welcome — open an issue to discuss ideas.
