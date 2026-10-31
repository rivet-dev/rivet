# 0002: caller-chosen checkpoint ID

**What it changes.** Adds `CheckpointOptions::checkpoint_id`. A repeated request with the same ID returns the original checkpoint, even after newer writes or a new writer, including when two requests meet in one flush batch. Touches `Admin::create_checkpoint`, `Db::create_checkpoint`, both manifest-store checkpoint writers and the memtable flusher's manifest batch.

**Needed by.** Split and move: the coordinator persists the checkpoint ID in its intent before acting, and `final_checkpoint` passes it, so a retry finds the original cut. Tests: `engine/packages/env/tests/slatedb_checkpoint.rs`.

**Upstream.** The two-phase checkpoint (`begin_checkpoint` plus `refresh_checkpoint`, PR #2084, merged, unreleased) gives the same guarantee from the other side.

**Drop when.** Depot moves to the two-phase checkpoint on 0.18.
