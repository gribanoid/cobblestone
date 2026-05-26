# 🗺 Cobblestone Roadmap

Items are roughly ordered by priority within each milestone.

---

## ✅ v0.1 — Foundation *(current)*

- [x] Plain Markdown storage in `~/.cobblestone`
- [x] `cobblestone-core` shared library (workspace crate)
- [x] CLI (`cb`) — `ls`, `new`, `show`, `edit`, `rm`, `search`
- [x] `-i` flag — full-screen TUI (ratatui + crossterm)
- [x] `cb web` — embedded local web server (axum) with browser UI
- [x] Web UI — dark/light theme, live Markdown preview, auto-save
- [x] Desktop app scaffold — Leptos 0.8 CSR + Tauri 2 backend
- [x] Tauri IPC commands: list, get, save, create, delete, search
- [x] Tag extraction (`#tag` syntax), full-text search
- [x] GNU GPL v3 license

---

## 🔧 v0.2 — Polish & Power User

- [ ] **Desktop app launch** — fully functional Leptos UI with Tauri IPC wired up
- [ ] **Wikilinks** — `[[Note Title]]` creates clickable cross-note links
- [ ] **Backlinks panel** — see all notes that link to current note
- [ ] **Daily note** — `cb today` opens/creates a note for today's date
- [ ] **Note templates** — `cb new --template meeting`
- [ ] **Pinned notes** — appear at top of list
- [ ] **Configurable storage** — `CB_DIR` env var or `~/.config/cobblestone/config.toml`
- [ ] **Shell completions** — bash, zsh, fish via `clap_complete`
- [ ] **TUI inline editing** — edit without leaving `cb -i`
- [ ] **Mouse support** in TUI

---

## 🖥 v0.3 — Desktop App Full Feature

- [ ] **Native file watcher** — reload note list when files change on disk
- [ ] **Graph view** — force-directed link graph between notes
- [ ] **Split editor/preview** — side-by-side in desktop window
- [ ] **Drag-and-drop attachments** — images stored in `~/.cobblestone/assets/`
- [ ] **Native menu bar** — File / Edit / View menus
- [ ] **System tray** — quick access to create note from anywhere
- [ ] **Native notifications** — remind to open daily note
- [ ] **Custom CSS themes** — drop `~/.cobblestone/theme.css` to restyle

---

## 🌐 v0.4 — Web UI Enhancements

- [ ] **CodeMirror editor** — syntax highlighting, bracket matching
- [ ] **Real-time wikilink navigation** — `[[note]]` clickable in preview
- [ ] **Full-text search** with highlighted matches
- [ ] **Folder/category support** — subdirectories in storage
- [ ] **PWA manifest** — installable as desktop app from browser
- [ ] **Offline-first service worker**

---

## 🔔 v0.5 — Notifications & Time-based Events

- [ ] **Due dates** — add `due: 2026-06-01` to any note's front-matter; `cb` picks it up automatically
- [ ] **Reminders** — `cb remind <slug> <time>` schedules a one-shot system notification (`notify-send` / `osascript` / Windows toast)
- [ ] **Recurring events** — cron-like syntax in front-matter: `repeat: every Monday 09:00`
- [ ] **Overdue warnings** — `cb` and TUI highlight notes with passed due dates in red
- [ ] **Agenda view** — `cb agenda` shows all notes with upcoming due dates sorted by time
- [ ] **Event daemon** — optional background process (`cb daemon`) that watches for due events and fires notifications without the app being open
- [ ] **Calendar export** — `cb export --ical` generates an `.ics` file compatible with any calendar app
- [ ] **Native desktop notifications** — Tauri plugin for OS-level alerts (macOS Notification Center, Linux libnotify, Windows toast)
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

- [ ] Homebrew formula (`brew install cobblestone`)
- [ ] Pre-built binaries: macOS arm64/x86_64, Linux x86_64, Windows x86_64
- [ ] GitHub Actions CI — build, test, release pipeline
- [ ] AUR package (Arch Linux)
- [ ] Flatpak (Linux)
- [ ] `cargo-binstall` support

---

## 💡 Ideas Backlog

- Mobile companion app (Tauri mobile target — iOS/Android)
- Browser extension to clip web pages as notes
- CRDT-based local sync (Automerge)
- Vim key-bindings mode in web editor
- YAML front-matter support (`---` header parsing)
- Note version history (per-note git blame)

---

Contributions welcome — open an issue to discuss ideas.
