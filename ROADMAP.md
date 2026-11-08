# Cobblestone Roadmap

Items are roughly ordered by priority. Product stays **MIT**. See [README](README.md), [NOTICE](NOTICE), [docs/plugin-bridge.md](docs/plugin-bridge.md).

**Current focus:** wire plugins into everyday UX (search, tasks, settings) + vault reliability (watcher, wikilinks).

---

## v0.1 — Foundation

- [x] Plain Markdown storage + `cobblestone-core`
- [x] CLI (`cb`), TUI (`-i`), `cb web`
- [x] Desktop scaffold (Dioxus)
- [x] Tags, full-text substring search, MIT license

---

## v0.2 — Polish & power user *(shipped core UX)*

- [x] Shared Dioxus UI (desktop + web WASM)
- [x] Nested file tree, CRUD, DnD, context menus
- [x] Info panel: tags, wikilinks, backlinks
- [x] CodeMirror source editing and optional Rust Live Preview beta
- [x] Attachments / `![[image]]` paste

- [x] Clickable `[[wikilinks]]` in Reading view
- [ ] Backlink line highlights
- [ ] Pinned notes; shell completions; TUI folder tree / mouse

---

## v0.3 — Desktop shell

- [x] Right info panel
- [ ] Native file watcher + UI reload
- [ ] Graph view (force-directed)
- [ ] Split editor/preview
- [ ] Native menu / tray / notifications
- [ ] Custom CSS themes

---

## v0.4 — Web

- [ ] Editor polish on WASM (clipboard parity)
- [ ] Search match highlights
- [x] Nested folders
- [ ] PWA / offline SW

---

## v0.5–v0.7 — Time, sync, intelligence

- [ ] Due dates / agenda / daemon (overlaps Tasks)
- [ ] Git sync, export, encrypted notes
- [ ] Local LLM / semantic search / spaced repetition

---

## v0.8 — Plugin platform *(mostly shipped)*

First-party Rust plugins only (no foreign `main.js`). Copyleft upstream sources are never vendored.

| Decision | Choice |
|----------|--------|
| API | `cobblestone-plugin-api` + `compat` |
| Id/path map | `cobblestone-plugin-bridge` |
| Load v0 | Static builtins via `plugin-host` |
| Dynamic `.dylib` | Later milestone + license gate |

Shipped:

- [x] plugin-api / bridge / host
- [x] Plugins: `periodic-notes`, `tasks`, `quick-capture`, `dynamic-templates`, `omni-search`
- [x] CLI: `cb plugin`, `today`, `omni`, `project`
- [x] Desktop: ⌘P palette, Reading post-processors, projects modal, editor modes ⌘E

---

## Per-crate roadmap *(reconciled)*

### `cobblestone-core`

**Shipped:** nested Store, attachments, tags/wikilinks/`note_graph`, `ProjectsConfig` + `CB_DIR` / `projects.toml`.

**Next:**

1. Vault file-watcher / change events
2. Incremental search index hooks (invalidate via watcher)
3. Wikilink harden: `[[Note\|alias]]`, `#heading`, resolve by title
4. Rewrite inbound links on rename/move
5. Soft-delete / trash
6. Shared backup/restore API (lift from CLI)

### `cb` (CLI)

**Shipped:** ls/new/show/edit/rm/search, TUI, web, backup/restore, plugin + project commands.

**Next:**

1. Folder CLI parity (`mkdir` / `mv` / tree)
2. Attachment CLI
3. TUI: folder tree + Omni/backend search
4. `plugin run` open `$EDITOR` like `today`
5. Shell completions; `--project` flag
6. Project-aware backup defaults

### `cobblestone-editor`

**Shipped:** block Live Preview, session TDD, embeds, desktop paste.

**Next:**

1. Source ↔ LP parity (IME, selection)
2. Navigation hooks for `[[wikilinks]]`
3. Tables / callouts / front matter (incremental)
4. WASM clipboard path
5. Optional LP post-processors

### `cobblestone-ui`

**Shipped:** tree/CRUD/DnD, modes, ⌘P, projects, attachments, desktop host.

**Next:**

1. Graph view and reload on filesystem changes
2. Add built-in actions and keyboard navigation to the command palette
3. Open projects with the OS folder picker
4. Show plugin availability in the web interface

### `cobblestone-desktop`

**Shipped:** thin launcher → `launch_desktop` + in-process host.

**Next:** watcher shell, menu/tray, window state, deep links to running app — keep UI in `cobblestone-ui`.

### `cobblestone-plugin-api`

**Next:** settings-tab API, editor selection and cursor on `App`, and vault events.

### `cobblestone-plugin-bridge`

**Next:** one-shot import of upstream `data.json` (read-only `.obsidian`); keep all upstream ids only here.

### `cobblestone-plugin-host`

**Next:** enable or disable plugins from desktop settings and invalidate cached metadata.

### Plugins

| Id | Next priorities |
|----|-----------------|
| `tasks` | [x] `tasks:toggle`; click in UI; recurrence on complete; query parity; index cache |
| `quick-capture` | `{{VALUE}}` prompt; SELECTION/CLIPBOARD; Multi menu; settings UI |
| `periodic-notes` | settings tab; weekly/monthly next/prev |
| `dynamic-templates` | picker; insert at caret + cursor marker in editor |
| `omni-search` | [x] desktop sidebar; persistent/incremental index; ⌘O |

---

## Active implementation queue

1. [x] Omni search in desktop sidebar (when plugin host present)
2. [x] `tasks:toggle` command (palette)
3. [x] Clickable wikilinks in Reading view
4. [x] Rust plugin host bridges: prompt / editor insert / notices / tasks panel
5. Core watcher API (foundation for UI reload)

---

## Distribution

- [x] CI release binaries (unsigned)
- [ ] Homebrew, notarization, AUR, Flatpak

### macOS Gatekeeper

```bash
xattr -cr /Applications/Cobblestone.app
```

---

## Ideas backlog

Mobile, browser clipper, CRDT sync, Vim bindings, YAML front-matter, licensed dynamic plugins.

---

Contributions welcome — open an issue to discuss ideas.
