# Plugin runtime (Rust-only)

Cobblestone runs **first-party Rust plugins** only. Upstream community
JavaScript trees in `../cobblestone-plugin-forks/` are **behavior references**,
not a loadable plugin format.

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────────┐
│ cobblestone │────▶│ plugin-host      │────▶│ plugins/* (Rust)    │
│ ui / cli    │     │ StoreVault       │     │ tasks, quick-capture│
└─────────────┘     │ SimpleEditor     │     │ periodic-notes, …   │
                    │ PromptBridge     │     └─────────────────────┘
                    └──────────────────┘
                              ▲
                              │ study only (no JS load)
                    ┌──────────────────┐
                    │ plugin-forks/    │
                    │ mit + copyleft   │
                    └──────────────────┘
```

## Host bridges

| Bridge | Role |
|--------|------|
| `ask_value` / `PromptBridge` | Desktop modal before command; CLI uses default/empty |
| `EditorApi` | Clipboard and caret insertion in desktop UI. Selection integration is pending. |
| `NoticeLog` | Toast in desktop UI and output from `cb plugin run` |
| `emit_ui` | `focus-search`, `show-tasks` |

## Commands (⌘P / `cb plugin run`)

See `cb plugin commands`. Notable:

- `tasks:list` / `tasks:toggle`
- `quick-capture:choice-*` (prompts when `{{VALUE}}` needed)
- `dynamic-templates:insert` / `:new`
- `omni-search:open` / `:reindex`
- `periodic-notes:open-*` / `next` / `prev`
