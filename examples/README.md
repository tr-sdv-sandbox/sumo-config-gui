# SUMO Target Config GUI examples

This directory is the default config root used by `scripts/run.sh`.

It intentionally contains one example for each supported GUI schema:

- `v1/channels/bleeding/vehicle.json` — legacy `vehicle.json` schema used by existing managed CVC tower channel folders.
- `v2/truck-002.yaml` — proposed target-type/profile-oriented YAML schema from `docs/sumo-provision-multi-profile-update-draft.md`, including nested HPC workloads.

Use these files for local file-mode smoke testing only. Clone/disable actions performed from the GUI will write next to these examples unless you point **Local test config root** at another directory.
