# `cobblestone-plugin-bridge`

Maps upstream plugin identifiers and vault layout onto Cobblestone.

```text
upstream id / path  →  bridge  →  cobblestone-plugin-api / host
```

## API highlights

| Function | Role |
|----------|------|
| `to_cobblestone_plugin_id` | `obsidian-tasks` → `tasks` |
| `to_cobblestone_command_id` | `obsidian-tasks:list` → `tasks:list` |
| `normalize_enabled_plugin_ids` | migrate `plugins.json` lists |
| `plugin_data_path` | `{vault}/.cobblestone/plugins/{id}/data.json` |
| `upstream_plugin_data_path` | read-only import from `.obsidian/plugins/…` |
| `to_note_id` / `to_vault_md_path` | note id ↔ `*.md` path |

Upstream names live **only** in this crate’s tables. Public Cobblestone ids stay short kebab-case.

Does **not** execute JavaScript plugins; it only translates entities for Rust ports and host.
