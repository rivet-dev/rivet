# Workflow Engine Notes

## Dirty State Requirements

- History entries must set `entry.dirty = true` whenever the entry is created or mutated. `flush()` persists dirty entries and clears the flag.
- Entry metadata must set `metadata.dirty = true` whenever metadata fields change. `flush()` persists dirty metadata and clears the flag.
- Name registry writes are tracked by `storage.flushedNameCount`. New names must be registered with `registerName()` before flushing.
- Workflow state/output/error are tracked via `storage.flushedState`, `storage.flushedOutput`, and `storage.flushedError`. Update the fields and call `flush()`; it will write if the value changed.
- `flush()` does not clear workflow output/error keys when values are unset. If you need to clear them, explicitly `driver.delete(buildWorkflowOutputKey())` or `driver.delete(buildWorkflowErrorKey())`.
