# Workflow Engine Guidance

## Persist Schema Sync

The workflow engine persistence schema is duplicated in RivetKit for inspector transport.
When updating `schemas/v1.bare` in this package, you must update the mirror at:

- `rivetkit-typescript/packages/rivetkit/schemas/persist/v1.bare`

After updating both, rebuild schemas:

- `pnpm -C rivetkit-typescript/packages/workflow-engine run compile:bare`
- `pnpm -C rivetkit-typescript/packages/rivetkit run build:schema`
