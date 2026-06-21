# 🪨 Cobblestone

## Sharpen your thinking.

**The open-source knowledge base and note manager for your private thoughts.**

> Your notes are yours. Cobblestone stores everything locally as plain Markdown files — on your device, in your directory, under your control.

Use it as a **personal knowledge base**, a **todo list**, a **daily journal**, or a fast place to capture thoughts.

**Status:** v0.2 in progress — see [ROADMAP.md](ROADMAP.md) for what's shipped and what's next.

---

## Quick start — desktop

Native window via **Vite** and **Tauri 2** — no Electron, no browser required.

```bash
git clone https://github.com/yourname/cobblestone
cd cobblestone
```

**Prerequisites (one-time):**

```bash
node --version       # 18+
make npm-install
cargo install tauri-cli --version "^2.0.0" --locked
xcode-select --install   # macOS only, if needed
```

**Run / build:**

```bash
make desktop         # dev: Vite on :1420 + native window
make desktop-build   # release bundle
npm run dev:desktop  # frontend only, for UI iteration
```

Release artifacts (after `make desktop-build`):

- **macOS** → `target/release/bundle/macos/Cobblestone.app`
- **Linux** → `target/release/bundle/deb/*.deb`, `*.AppImage`
- **Windows** → `target/release/bundle/msi/*.msi`

Run `make help` for all build and dev commands.

---

## Why Cobblestone?

| | Cobblestone |
|---|---|
| **Storage** | Plain `.md` files in `~/.cobblestone` (nested folders supported) |
| **Privacy** | 100% local — nothing leaves your machine |
| **Interfaces** | Native desktop app · Browser UI · Terminal CLI |
| **License** | MIT |
| **Lock-in** | None — edit files with any text editor |
| **Cost** | Free, always |

---

## Graphical UI

The desktop app and browser UI share the same **TypeScript** UI (`@cobblestone/ui`). Both talk to `cobblestone-core` — Tauri `invoke` on desktop, REST `/api/*` in the browser.

**Features:**

- Nested file tree with expand/collapse
- Folders — create, rename, move, delete (with confirmation)
- Notes — create, rename, move, delete; drag-and-drop in the tree
- Context menus (right-click on notes and folders)
- Search, edit/preview toggle, autosave
- Dark/light theme
- Info panel — tags, metadata, outgoing wikilinks, backlinks (clickable)

Wikilinks (`[[Note Title]]`) are parsed in core and shown in the info panel. Clicking a link in the **preview body** is [planned](ROADMAP.md).

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` / `⌘S` | Save current note |
| `Tab` (in editor) | Insert 2 spaces |
| `Esc` | Close modal or context menu |

### Web — `cb web`

```bash
make npm-install     # once
make web-build       # build the shared UI into frontend/apps/web/dist
make web             # or: cargo run -p cb -- web
```

Open `http://127.0.0.1:3000` if the browser doesn't open automatically.

---

## Storage format

Notes are plain UTF-8 Markdown — readable and editable with any tool:

```
~/.cobblestone/
  shopping-list.md
  ideas/
    project-alpha.md
  journal/
    2026-05-27.md
```

- **Title** — first `# Heading` in the file
- **Slug** — derived from the title: `"My Note"` → `my-note`; nested notes use paths like `ideas/my-note`
- **Tags** — `#hashtag` in body text (not in headings)
- **Wikilinks** — `[[Other Note]]` for cross-references (resolved in the info panel)
- **Sync** — works with Git, Syncthing, rsync, or any file sync tool

### Example note

```markdown
# Shopping List

*Created: 2026-05-27*

- [ ] Milk
- [ ] Bread
- [x] Coffee  #groceries

Related: [[Meal Planning]]
```

---

## Architecture

```
cobblestone/
  frontend/
    packages/
      api/                # CobblestoneApi types + web/tauri adapters
      ui/                 # shared UI (CSS, app logic, rendering)
    apps/
      desktop/            # Vite entry → Tauri window
      web/                # Vite entry → axum static files
  Makefile
  crates/
    core/                 # cobblestone-core — storage, search, tree, wikilink graph
    cli/                  # cb binary (clap · ratatui · axum web server)
    desktop/
      src-tauri/          # Tauri 2 backend (thin IPC over core)
```

All interfaces read and write the same Markdown files. The graphical UI uses `CobblestoneApi` — thin adapters over the same `Store` methods (`list_tree`, `note_graph`, folder CRUD, etc.).

---

## CLI — `cb`

Terminal interface for scripting, SSH, and quick shell access.

```bash
cargo install --path crates/cli
```

```bash
# Basic usage
cb                        # list all notes (recursive, flat)
cb new "Shopping List"    # create a note (opens $EDITOR)
cb show shopping-list     # pretty-print a note to stdout
cb show ideas/project     # nested notes use path-style slugs
cb edit shopping-list     # open a note in $EDITOR
cb rm  shopping-list      # delete a note (asks for confirmation)
cb search "rust async"    # full-text search across all notes

# Directory listing
cb ls                     # same as cb — lists all notes
cb ls ideas/              # list files and folders in a subdirectory

# Web UI
cb web                    # opens http://127.0.0.1:3000
cb web --port 8080        # custom port
```

Notes in subfolders appear in `cb ls` and search with their full slug (e.g. `ideas/project-alpha`).

### Interactive TUI — `cb -i`

Flat list of all notes (newest first) with live preview. Folder tree navigation is on the [roadmap](ROADMAP.md).

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
| `D` | Delete note (with confirmation) |
| `/` | Live search / filter |
| `^D` / `^U` | Scroll preview pane |
| `q` | Quit |

---

## License

MIT License — see [LICENSE](LICENSE).
