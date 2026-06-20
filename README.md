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

### Developer shortcuts

From the repository root you can use `make` for the common local flows:

```bash
make desktop        # run the native Tauri desktop app
make desktop-build  # build the desktop release bundle
make web            # run the browser UI
make tui            # run the terminal UI
make cli ARGS='ls'  # run any cb command
make test           # run workspace + desktop tests
```

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

## Desktop app — TypeScript + Tauri

A native desktop window built with **TypeScript**, **Vite**, and **Tauri 2** — lightweight, no Electron, no browser required. The UI talks to the same `cobblestone-core` storage layer as the CLI and web UI.

### Prerequisites (one-time setup)

```bash
# 1. Node.js 18+ and npm
node --version
npm --version

# 2. Install frontend dependencies
cd crates/desktop
npm install

# 3. Install Tauri CLI v2
cargo install tauri-cli --version "^2.0.0" --locked

# 4. macOS only — install Xcode Command Line Tools if not present
xcode-select --install
```

### Run in development

From the repository root:

```bash
make desktop
```

This starts Vite (hot-reload on port 1420) and opens the native window automatically.

You can also run the frontend alone for quick UI iteration:

```bash
cd crates/desktop
npm run dev
```

### Build a release binary

```bash
make desktop-build
```

Or manually:

```bash
cd crates/desktop/src-tauri
cargo tauri build
```

Output artifacts:
- **macOS** → `target/release/bundle/macos/Cobblestone.app`
- **Linux**  → `target/release/bundle/deb/*.deb` and `*.AppImage`
- **Windows** → `target/release/bundle/msi/*.msi`

### Desktop shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` / `⌘S` | Save current note |
| `Tab` (in editor) | Insert 2 spaces |
| `Esc` | Close delete confirmation dialog |

Features: sidebar note list, search, edit/preview toggle, autosave, metadata panel (tags, wikilinks, backlinks).

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
  Cargo.toml              # workspace (core, cli, desktop backend)
  Makefile                # make desktop / web / tui / test
  crates/
    core/                 # cobblestone-core — shared storage library
    cli/                  # cb binary (clap · ratatui · axum)
    desktop/
      src/                # TypeScript frontend (Vite · marked)
      src-tauri/          # Tauri 2 native backend (Rust IPC commands)
      index.html          # app shell and styles
      package.json        # npm scripts: dev, build
```

The desktop frontend calls Tauri commands (`list_notes`, `get_note`, `save_note`, etc.) defined in `src-tauri/src/commands/`. All interfaces share the same Markdown files in `~/.cobblestone`.

---

## License

GNU General Public License v3.0 — see [LICENSE](LICENSE).

Free as in freedom, forever.
