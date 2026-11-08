---
name: cobblestone-trademarks
description: >-
  Trademark and naming rules for Cobblestone plugins and docs. Avoid third-party
  brand confusion; use cobblestone_* ids. Use when editing plugin-api, plugins,
  NOTICE, README branding, or crates.io descriptions.
---

# Cobblestone trademarks & naming

## Product identity

- Public product name: **Cobblestone** only.
- Do not market features as «X-compat» using another product’s trademark as the
  feature brand (prefer «plugin platform», «vault projects», «editor modes»).
- Do not use third-party logos or app icons.

## Allowed references

- Factual attribution in `NOTICE` and internal docs (upstream repo URLs, licenses).
- Nominative phrases like «not affiliated with …» in README/NOTICE.
- Studying MIT-licensed upstream sources in gitignored `third_party/` for ports.

## Forbidden / high-risk

- Product, crate, or crates.io names containing another app’s trademark.
- Plugin `id` values containing foreign brand tokens (use `tasks`, not brand-tasks).
- Module names like `*_compat` tied to a foreign brand — use `compat` / `plugin_api`.
- Claiming Cobblestone runs unmodified foreign `main.js` plugins.
- Copying proprietary app binaries or Electron sources.
- Vendoring AGPL/GPL plugin source into this MIT tree.

## Cobblestone ids (canonical)

| Kind | Pattern | Examples |
|------|---------|----------|
| Crate | `cobblestone-*` | `cobblestone-plugin-api` |
| Plugin id | short kebab | `tasks`, `quick-capture`, `periodic-notes`, `dynamic-templates`, `omni-search` |
| Command id | `{plugin}:{action}` | `tasks:list`, `periodic-notes:open-daily` |
| Settings | `{vault}/.cobblestone/plugins/{id}/data.json` | |
| Enable list | `{vault}/.cobblestone/plugins.json` | |

## Future forks

When forking public MIT API type definitions into the repo, place them under
`crates/` or `third_party/cobblestone-api-types/` with LICENSE + NOTICE update.
Keep Cobblestone naming on the public surface; keep upstream provenance in NOTICE.

## Id / path bridging

Use crate `cobblestone-plugin-bridge` for all upstream ↔ Cobblestone id and path
translation. Do not scatter upstream plugin ids across host/UI.
