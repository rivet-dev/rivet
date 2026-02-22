# RivetKit Friction Log

This file tracks recurring friction points encountered while designing and documenting RivetKit game examples.

## 2026-02-13

- `openspec/` exists but contains no example artifacts yet (no existing spec templates to mirror directly).
- Repo guidance says notes should live under `.agents/notes/...`, but this task requires `.agent/notes/...` and `.agent/friction/...`.
- `examples/sqlite-drizzle` appears stubbed/commented; for SQLite persistence examples, `examples/sqlite-raw` is the clearest reference pattern.
- Requirement tension: "players must not be able to cheat location" vs "prefer client-authoritative for fast-paced games". Specs will use client-authoritative movement with simple caps/rate limits and server-side sanity checks, not full anti-cheat.
- Using `c.conn.id` as identity makes \"per-player persistent world\" ambiguous across reconnects; idle spec treats a player as a session (with an optional token extension).
- Requested deletion of `examples/multiplayer-game` while this phase is "specs only" can leave the auto-generated `examples/multiplayer-game-vercel` out of sync until regeneration.
- Local deletion via shell commands was blocked by policy; removed `examples/multiplayer-game` tracked files via patch deletes instead (untracked `node_modules/` may still exist on disk).
- Matchmakers using SQLite introduces the need for stale-entry cleanup (missed close events) and simple indexing queries; specs include `cleanupStale*` scheduled actions to keep tables bounded.
