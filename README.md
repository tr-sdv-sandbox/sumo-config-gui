# SUMO Target Config GUI

Desktop GUI for viewing and maintaining local SUMO target configuration files.

- Tauri 2 + React + TypeScript frontend
- Rust model/repository/controller backend exposed through GUI-internal Tauri IPC
- Tower 2 target releases as the normal source of truth
- Target-type/profile-aware SUMO configuration browsing
- Read-only Tower 2 target release linkage checks

## Quick start

From this repository root:

```bash
mise run run
```

To start the GUI and the local software tower together from the workspace root, use:

```bash
../../scripts/run-config-gui-stack.sh
```

The workspace launcher automatically selects a free Tower 2 port when `8081` is already occupied and passes the correct Tower 2 URL to the GUI.

Or without mise:

```bash
npm install
./scripts/run.sh
```

The app starts in Tower 2 mode and lists target releases from the configured software tower. When a channel has a stored config snapshot, the detail view shows that authoring config alongside the resolved release tree. Local files are for smoke testing only.

To test local file mode with bundled examples:

```bash
SUMO_CONFIG_GUI_ROOT=$PWD/examples ./scripts/run.sh
```

The bundled examples include both supported file schemas:

```text
examples/v1/channels/bleeding/vehicle.json
examples/v2/truck-002.yaml
```

For a quick file-mode smoke test, confirm both example target configs appear, expand their details, clone one, and disable the clone.

## Start Tower 2

Target release linkage badges use the local `sumo-provision` software tower. Start the local tower stack in a separate terminal; the GUI only uses Tower 2:

```bash
cd ../sumo-provision
./start.sh
```

Default Tower 2 URL:

```text
Tower 2 / software-tower: http://localhost:8081
```

Quick health check:

```bash
cargo run -p cli -- hub ping
```

If Tower 2 is not running, set `SUMO_CONFIG_GUI_ROOT` to use local file-mode smoke tests. Target release badges will show unavailable or skipped.

Stop the tower stack with `Ctrl-C` in the `./start.sh` terminal.

## Build a desktop bundle

```bash
mise run build
```

Or without mise:

```bash
./scripts/build.sh
```

## Validate

```bash
mise run validate
```

## Example configuration files

The `examples/` directory is meant only for local file-mode smoke testing:

- `examples/v1/channels/bleeding/vehicle.json` uses the current channel `vehicle.json` schema.
- `examples/v2/truck-002.yaml` uses the proposed target-type/profile-oriented YAML schema with nested HPC workloads.

Use Tower 2 mode for normal target release browsing. Use another config root only when intentionally testing local draft files so GUI clone/disable actions do not modify the bundled examples.

## Related examples in the workspace

Besides this repository's bundled `examples/`, the workspace contains these useful example locations:

- `../../examples/t2-seed-bleeding/channels/*/vehicle.json` — real current `vehicle.json` examples used by the `t2-seed-*` tower seeders.
- `../../examples/managed-cvc` — managed CVC rig configs and policy examples.
- `../../examples/managed-qemu` and `../../examples/campaign` — QEMU/campaign examples.
- `../SOVDd/crates/sovd-client/examples` — SOVD client flash config YAML examples.
- `../sumo-provision/crates/software-tower/examples` — Tower 2 Rust example/benchmark code.
- `../sumo-machine-manager/example` — machine-manager factory and VM config examples.
- `../supernova-machine-manager/examples` — QNX/supernova example configs and scripts.

For GUI schema testing, start with this repository's `./examples` directory. For realistic SUMO data, use Tower 2 mode rather than pointing the GUI at private population files.
