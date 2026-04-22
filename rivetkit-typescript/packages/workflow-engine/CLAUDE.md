# Workflow Engine Guidance

- `storage.flush(...)` chunks driver batch writes to actor KV limits (128 entries / 976 KiB payload) and clears dirty markers only after all write/delete operations succeed.

## Persist Schema Sync

- The workflow engine persistence schema is duplicated in RivetKit for inspector transport.
- When updating `schemas/v1.bare` in this package, also update the mirror at `rivetkit-typescript/packages/rivetkit/schemas/persist/v1.bare`.

- After updating both schema files, rebuild schemas with the commands below.

- `pnpm -C rivetkit-typescript/packages/workflow-engine run compile:bare`
- `pnpm -C rivetkit-typescript/packages/rivetkit run build:schema`
