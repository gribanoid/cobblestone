# Plugin API surface v0

Rust crate: `cobblestone-plugin-api` (module `compat`).

First-party host types for statically linked Rust plugins. Upstream API
references are listed in [`NOTICE`](../NOTICE).

## Core types

| Concept | Rust (v0) | Notes |
|---------|-----------|-------|
| Plugin | `trait Plugin` | `onload` / `onunload`, manifest |
| Component | `trait Component` | children, `register` cleanup |
| Manifest | `struct PluginManifest` | id, name, version, min_app_version |
| App | `struct App` | vault, workspace, metadata, commands |
| Vault | `trait VaultApi` | CRUD + paths |
| File / folder | `TFile` / `TFolder` | path-relative to vault root |
| Workspace | `trait WorkspaceApi` | active file, open path |
| Metadata | `trait MetadataCacheApi` | links, tags, headings |
| Command | `struct Command` | id, name, callback |
| Post-processor | `MarkdownPostProcessor` | HTML hook for Reading view |
| Notice | `fn notice(msg)` | host toast |
| Ask value | `ask_value(prompt, default)` | modal / CLI |
| Editor | `EditorApi` | selection, clipboard, queue_insert |
| Editor mode | `EditorMode` | LivePreview / Source / Reading |
| UI hints | `WorkspaceApi::emit_ui` | `focus-search`, `show-tasks` |

## Settings persistence

- Plugin data: `{vault}/.cobblestone/plugins/{id}/data.json`
- Enabled plugins: `{vault}/.cobblestone/plugins.json`

## Out of scope for v0

- Loading third-party `.dylib` or JavaScript plugins
- Full CodeMirror-style editor extensions
