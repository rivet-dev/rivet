# Depot Tier Test Matrix Results

Run at: 2026-05-01T15:28:56-07:00

Command:

```bash
cargo test -p depot
```

Result: passed.

Summary:

```text
lib unit tests: 28 passed
burst_mode: 1 passed
cold_tier: 4 passed
conveyer_branch: 19 passed
conveyer_commit: 15 passed
conveyer_compaction_payloads: 7 passed
conveyer_constants: 1 passed
conveyer_error: 2 passed
conveyer_keys: 13 passed
conveyer_page_index: 4 passed
conveyer_pitr_interval: 2 passed
conveyer_policy: 4 passed
conveyer_quota: 4 passed
conveyer_read: 19 passed
conveyer_restore_point: 26 passed
debug: 3 passed
fork_bucket: 4 passed
fork_common_helpers: 1 passed
fork_database: 6 passed
gc: 4 passed
gc_pin_recompute_under_restore_point_delete_race: 1 passed
list_databases: 3 passed
restore_points: 5 passed
takeover: 1 passed
workflow_compaction_payloads: 1 passed
workflow_compaction_skeletons: 69 passed
doc tests: 0 passed
```

Exit status: `0`

Notes:

- The first combined run failed to compile because existing Depot source edits had dropped `anyhow::Error` from `conveyer/commit/helpers.rs` and borrowed temporary `tx.informal()` handles across `try_join!` in `burst_mode.rs`.
- Those compile blockers were fixed narrowly, then the command above passed.
- `workflow_compaction_skeletons.rs` now runs its tier-agnostic manager, hot compaction, PITR, and reclaim cases through a local disabled/filesystem workflow matrix. Cold-only and cold-disabled assertions remain single-mode by design.
