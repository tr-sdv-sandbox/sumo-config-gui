# SUMO Config GUI Index

Tauri desktop GUI for browsing SUMO target configs and Tower 2 target releases, with local file-mode smoke tests.

## Where to look

- `README.md` — run/build/validate flow and user-facing behavior.
- `../../docs/sumo-config-gui-architecture.md` — MVC design, schema adapter model, Tower 2 linkage boundaries.
- `src/App.tsx` — React UI, table/details, clone/disable actions, Tower status badges.
- `src-tauri/src/controller.rs` — Tauri IPC command surface.
- `src-tauri/src/repository.rs` — config discovery and adapter dispatch.
- `src-tauri/src/adapters/` — supported local file schemas: `vehicle.json` and profile YAML.
- `src-tauri/src/tower.rs` — Tower 1/2 HTTP lookup and linkage checks.
- `examples/` — bundled local file-mode smoke data only.

## Essential commands

Run from this component root unless noted; `mise` is the base path.

```bash
mise run run                         # dev desktop app
mise run build                       # Tauri desktop bundle
mise run validate                    # scripts + frontend build + Rust fmt/test/clippy
SUMO_CONFIG_GUI_ROOT=$PWD/examples mise run run
```

Workspace stack launcher:

```bash
cd ../..
./scripts/run-config-gui-stack.sh
```

Finding commands:

```bash
rg --files -g 'AGENTS.md' -g 'README*' -g 'Cargo.toml' -g 'package.json' -g 'mise.toml'
rg -n "tauri::command|invoke\(|SUMO_CONFIG_GUI_|Tower 2|vehicle.json|ProfileYaml|clone|disable" src src-tauri scripts examples
```

## Stack

- Tauri 2 backend in Rust 2021 with `serde`, `serde_json`, `serde_yaml`, `reqwest`, `tokio`, `thiserror`.
- React 18 + TypeScript + Vite frontend.
- npm lockfile for JS dependencies; Cargo lockfile under `src-tauri/`.

## Guardrails

- Tower 2 is the normal source of truth; local files are smoke-test/draft data.
- GUI IPC commands are internal to the desktop app, not a public CLI/API contract.
- Disable means mark inactive/disabled; do not implement hard delete unless the architecture changes.
- Keep schema differences inside `SchemaAdapter` implementations and return normalized `VehicleConfig` to the UI.
- Do not commit generated `node_modules/`, `dist/`, `tsconfig.tsbuildinfo`, or `src-tauri/target/` artifacts.

## Gotchas

- `scripts/run.sh` installs npm dependencies if `node_modules` is missing, then runs `npm run tauri -- dev`.
- `SUMO_CONFIG_GUI_ROOT` enables local file mode and clone/disable writes next to that root; avoid pointing it at shared example data unless intentionally testing writes.
- `../../scripts/run-config-gui-stack.sh` chooses a free Tower 2/HUB port and passes URLs to the GUI.
- `src-tauri/Cargo.toml` pins `time = "=0.3.36"` for the current Tauri/cookie toolchain.

## Missing docs/specs to watch

- The profile YAML shape comes from `../../docs/sumo-provision-multi-profile-update-draft.md`, which is a proposal with open questions.
- There is no standalone versioned schema document for the GUI's normalized `VehicleConfig`; the canonical shape is currently `src-tauri/src/model.rs` plus the architecture doc.
- No component-local CI/workflow file is present; `mise run validate` is the practical local quality gate.
