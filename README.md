# SUMO Config GUI

Desktop GUI for viewing and maintaining local SUMO vehicle configuration files.

- Tauri 2 + React + TypeScript frontend
- Rust model/repository/controller backend exposed through GUI-internal Tauri IPC
- Local-first config source of truth
- Read-only Tower 1/Tower 2 linkage checks

## Run the GUI

Install frontend dependencies once:

```bash
cd components/sumo-config-gui
npm install
```

Start the desktop app in development mode:

```bash
npm run tauri -- dev
```

The GUI auto-detects the default config root when possible:

```text
examples/managed-cvc-tower
```

You can override it in the **Config root** field.

## Start Tower 1 and Tower 2

Tower linkage badges use the local `sumo-provision` towers. Start them in a separate terminal:

```bash
cd components/sumo-provision
./start.sh
```

Default tower URLs:

```text
Tower 1 / identity-tower: http://localhost:8080
Tower 2 / software-tower:  http://localhost:8081
```

Quick health checks:

```bash
cargo run -p cli -- ca ping
cargo run -p cli -- hub ping
```

If the towers are not running, the GUI still works with local files. Tower linkage badges will show unavailable or skipped.

Stop the towers with `Ctrl-C` in the `./start.sh` terminal.

## Build a desktop bundle

```bash
cd components/sumo-config-gui
npm run tauri -- build
```
