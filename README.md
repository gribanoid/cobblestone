# 🪨 Cobblestone

## Sharpen your thinking.

**The open-source knowledge base and note manager for your private thoughts.**

> Your notes are yours. Cobblestone stores everything locally as plain Markdown files — on your device, in your directory, under your control. No cloud. No accounts. No subscriptions.

Use it as a **personal knowledge base**, a **todo list**, a **daily journal**, or just a fast place to capture thoughts from the terminal.

---

## Why Cobblestone?

| | Cobblestone |
|---|---|
| **Storage** | Plain `.md` files in `~/.cobblestone` |
| **Privacy** | 100% local — nothing leaves your machine |
| **Interfaces** | Terminal CLI · Browser UI · Native desktop app |
| **License** | GNU GPL v3 — forever free and open |
| **Lock-in** | None — edit files with any text editor |
| **Cost** | Free, always |

---

## Quick start

### 1. Install the CLI

```bash
git clone https://github.com/yourname/cobblestone
cd cobblestone
cargo install --path crates/cli
```

The binary is installed as **`cb`**.

### 2. Create your first note

```bash
cb new "My First Note"   # creates the file and opens $EDITOR
```

### 3. Pick your interface

| Interface | When to use |
|-----------|-------------|
| `cb` (terminal) | Fast access from any shell, scripting, SSH |
| `cb -i` (TUI) | Keyboard-driven full-screen experience |
| `cb web` (browser) | Rich editing with live Markdown preview |
| Desktop app | Native window, always open in the background |

---

## CLI — `cb`

All your notes at a glance, right in the terminal.

```bash
# Basic usage
cb                        # list all notes
cb new "Shopping List"    # create a note (opens $EDITOR)
cb new "Todo"             # works great as a todo list too
cb show shopping-list     # pretty-print a note to stdout
cb edit shopping-list     # open a note in $EDITOR
cb rm  shopping-list      # delete a note (asks for confirmation)
cb search "rust async"    # full-text search across all notes

# Directory listing
cb ls                     # same as cb — lists all notes
cb ls ideas/              # list a subdirectory

# Web UI
cb web                    # opens http://127.0.0.1:3000
cb web --port 8080        # custom port
```

### Running the interactive TUI

```bash
cb -i
```

```
┌── Notes (3) ──────┬── Shopping List ─────────────────────────────────┐
│ > Shopping List   │                                                   │
│   2026-05-27      │  # Shopping List                                  │
│                   │                                                   │
│   Project Ideas   │  - [ ] Milk                                       │
│   2026-05-26      │  - [ ] Bread                                      │
│                   │  - [x] Coffee                                     │
│   Daily Journal   │                                                   │
│   2026-05-25      │  *Last updated: 2026-05-27*                       │
└───────────────────┴───────────────────────────────────────────────────┘
 q:quit  n:new  e:edit  D:delete  /:search  j/k:navigate  ^D/^U:scroll
```

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `n` | Create new note |
| `e` | Edit selected note in `$EDITOR` |
| `D` | Delete note (with confirmation dialog) |
| `/` | Live search / filter |
| `^D` / `^U` | Scroll preview pane |
| `q` | Quit |

---

## Web UI — `cb web`

A browser-based editor with live Markdown preview, dark/light theme, and auto-save.

```bash
cb web              # starts server, opens browser automatically
cb web --port 8080  # choose a different port
```

Open `http://127.0.0.1:3000` if the browser doesn't open automatically.

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Save current note |
| `n` | New note (when not in editor) |
| `e` | Switch to editor mode |
| `/` | Focus search bar |
| `Tab` (in editor) | Insert 2 spaces |
| `Esc` | Close modal / blur input |

---

## Desktop app — Leptos + Tauri

A native desktop window built with **Leptos 0.8** + **Tauri 2** — lightweight, no Electron, no browser required.

### Prerequisites (one-time setup)

```bash
# 1. Install trunk — WASM bundler for Leptos
cargo install trunk

# 2. Install Tauri CLI v2
cargo install tauri-cli --version "^2.0.0" --locked

# 3. macOS only — install Xcode Command Line Tools if not present
xcode-select --install
```

### Run in development

```bash
cd crates/desktop
cargo tauri dev
```

This starts trunk (hot-reload WASM server on port 1420) and opens the native window automatically. Changes to Leptos components reload instantly.

### Build a release binary

```bash
cd crates/desktop
cargo tauri build
```

Output artifacts:
- **macOS** → `target/release/bundle/macos/Cobblestone.app`
- **Linux**  → `target/release/bundle/deb/*.deb` and `*.AppImage`
- **Windows** → `target/release/bundle/msi/*.msi`

---

## Storage format

Notes are plain UTF-8 Markdown files — readable and editable with any tool:

```
~/.cobblestone/
  shopping-list.md
  project-ideas.md
  daily-journal.md
```

- **Title** — first `# Heading` in the file becomes the display title
- **Slug** — derived from the title: `"My Note"` → `my-note.md`
- **Tags** — use `#hashtag` anywhere in the body text
- **Sync** — works with Git, Syncthing, rsync, or any file sync tool

### Example note

```markdown
# Shopping List

*Created: 2026-05-27*

- [ ] Milk
- [ ] Bread
- [x] Coffee  #groceries

> Tip: check store opens at 09:00
```

---

## Architecture

```
cobblestone/
  Cargo.toml              # workspace (crates/core, crates/cli)
  crates/
    core/                 # cobblestone-core — shared storage library
    cli/                  # cb binary (clap · ratatui · axum)
    desktop/
      src/                # Leptos 0.8 WASM frontend
      src-tauri/          # Tauri 2 native backend
```

---

## License

GNU General Public License v3.0 — see [LICENSE](LICENSE).

Free as in freedom, forever.
