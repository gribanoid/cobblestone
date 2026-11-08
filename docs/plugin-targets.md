# Plugin targets (behavior contracts)

First-party Rust plugins against `cobblestone-plugin-api`.
Cobblestone does **not** execute foreign community `main.js` bundles.

Versions, source links, and licenses for all upstream references are in
[`NOTICE`](../NOTICE). Runtime details are in [`plugin-runtime.md`](plugin-runtime.md).

Local reference checkouts live in `../cobblestone-plugin-forks/`. The optional
`scripts/refresh-pins.sh` in that directory refreshes them.

## Periodic Notes — `periodic-notes`

| Feature | v0 scope |
|---------|----------|
| Daily / weekly / monthly open-or-create | Yes |
| Folder + date format + template path settings | Yes |
| Next / previous when active note matches its daily format | Yes |
| CLI `cb today` → daily | Yes |

## Tasks — `tasks`

| Feature | v0 scope |
|---------|----------|
| Parse `- [ ]` / `- [x]` | Yes |
| Inline due / scheduled / repeat markers | Yes (documented subset) |
| `````tasks`` query blocks (status, path, due) | Yes (MVP filters) |
| Toggle done + basic recurrence | Yes (helpers) |

Supported inline markers (v0): `📅 YYYY-MM-DD` (due), `⏳ YYYY-MM-DD` (scheduled),
`🔁 <simple recurrence>` (e.g. `every day`, `every week`).

## Quick Capture — `quick-capture`

| Feature | v0 scope |
|---------|----------|
| Template / Capture / Macro / Multi | Yes (command macros, one level of Multi) |
| `{{DATE}}`, `{{VALUE}}`, `{{NAME}}`, clipboard | Yes |
| Selection placeholder | Parsed, with editor selection still to be wired |
| User JavaScript / AI providers | No |

## Dynamic Templates — `dynamic-templates`

| Feature | v0 scope |
|---------|----------|
| Date / time / title substitutions | Yes |
| Cursor position marker after insert | Parsed and removed from output. Caret placement is pending. |
| Create note from template / insert at cursor | Yes |
| Arbitrary JS / system commands | **No** |

## Omni search — `omni-search`

| Feature | v0 scope |
|---------|----------|
| Full-text index over notes + headings | Yes |
| camelCase / snake_case identifier tokens | Yes |
| Ranked results (BM25-like) | Yes |
| PDF / OCR | No |
