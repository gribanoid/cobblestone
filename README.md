# Cobblestone

## Sharpen your thinking.

**The open-source knowledge base and note manager for your private thoughts.**

> Your notes are yours. Cobblestone stores everything locally as plain Markdown files — on your device, in your directory, under your control.

Use it as a **personal knowledge base**, a **todo list**, a **daily journal**, or a fast place to capture thoughts.

**Status:** v0.8 plugin platform + editor modes + projects in progress — see [ROADMAP.md](ROADMAP.md).

---

## Quick start — desktop

Native window via **Dioxus** on top of `cobblestone-core` — full Rust stack, no Node/Electron.

```bash
git clone https://github.com/yourname/cobblestone
cd cobblestone
```

**Prerequisites (one-time):**

```bash
rustc --version      # stable (1.85+)
xcode-select --install   # macOS only, if needed
# Linux: WebKitGTK 4.1 + GTK 3 (see dioxus desktop docs)
make deps            # cargo fetch + dioxus-cli
```

**Run / build:**

```bash
make desktop         # run the native Dioxus desktop app
make desktop-build   # release binary: target/release/cobblestone
make macos-bundle    # macOS application: target/macos/Cobblestone.app
```

**GitHub release:** push a tag `v*` (e.g. `v0.1.0`) — CI builds platform binaries and attaches them to a draft [GitHub Release](https://github.com/gribanoid/cobblestone/releases).

**macOS download blocked?** Unsigned builds trigger Gatekeeper. See [ROADMAP — macOS troubleshooting](ROADMAP.md#macos-damaged-and-cant-be-opened) for the fix.

Run `make help` for all build and dev commands.

**CI checks** (also via `make ci` / `make lint`):

| Check | Tool |
|-------|------|
| Format | `cargo fmt --check` |
| Lints | `cargo clippy -D warnings` (desktop + wasm) |
| Tests | `cargo test` |
| Docs | `cargo doc -D warnings` |
| Advisories | `cargo deny` |
| Memory bugs (core) | AddressSanitizer on nightly |

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

Desktop and browser share the same **Dioxus** UI (`cobblestone-ui`). Desktop calls `cobblestone-core` in-process; the browser UI is WASM and talks to REST `/api/*` served by `cb web`.

**Features:**

- Nested file tree with expand/collapse
- Folders — create, rename, move, delete (with confirmation)
- Notes — create, rename, move, delete; drag-and-drop in the tree
- Context menus (right-click on notes and folders)
- Search, edit/preview toggle, autosave
- Dark/light theme
- Info panel — tags, metadata, outgoing wikilinks, backlinks (clickable)

CodeMirror is the default Markdown editor on desktop and web. Settings let you
choose line numbers, line wrapping, tab width, or the Rust Live Preview beta.
Use `Ctrl+E` / `⌘E` to switch between editing and reading.

To rebuild the bundled CodeMirror asset after changing its source, run
`npm ci` and `npm run build` in `crates/ui/codemirror`. Cargo uses the checked-in
bundle directly.

Wikilinks (`[[Note Title]]`) appear in the info panel and open matching notes from Reading view.

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` / `⌘S` | Save current note |
| `Tab` (in CodeMirror) | Indent using the configured width |

### Web — `cb web`

```bash
make deps            # once
make web-build       # build WASM UI into crates/ui/dist-web
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
  Makefile
  crates/
    core/          # cobblestone-core — storage, search, tree, wikilink graph
    cli/           # cb binary (clap · ratatui · axum web server)
    ui/            # cobblestone-ui — shared Dioxus UI (desktop + WASM web)
    desktop-app/   # cobblestone-desktop — native Dioxus window + DirectStore
```

All interfaces read and write the same Markdown files. The graphical UI uses a `VaultBackend` trait — `DirectStore` on desktop, HTTP client to `/api/*` in the browser — over the same `Store` methods (`list_tree`, `note_graph`, folder CRUD, etc.).

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
cb backup                 # zip vault → ~/Documents/CobblestoneVault-backups/
cb backup -o ./vault.zip  # custom output path (or a directory)
cb restore ./vault.zip    # restore; aborts on conflicts
cb restore ./vault.zip -f # restore and overwrite conflicting files

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

## Plugins & licensing

Cobblestone ships a **first-party Rust** plugin API (`cobblestone-plugin-api`). It does **not** load foreign JavaScript community plugins. Built-ins include Tasks, Quick Capture, Periodic Notes, Dynamic Templates, and Omni search.

Third-party names appear in [NOTICE](NOTICE) only for attribution and trademark disclaimer. See also [docs/plugin-targets.md](docs/plugin-targets.md) and [ROADMAP v0.8](ROADMAP.md).

---

## License

MIT License — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
