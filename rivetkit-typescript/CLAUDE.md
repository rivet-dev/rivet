# rivetkit-typescript/CLAUDE.md

## Tree-Shaking Boundaries

- Do not import `@rivetkit/workflow-engine` outside the `rivetkit/workflow` entrypoint so it remains tree-shakeable.
- Do not import SQLite VFS or `@rivetkit/sqlite` outside the `rivetkit/db` (or `@rivetkit/sqlite-vfs`) entrypoint so SQLite support remains tree-shakeable.
- Importing `rivetkit/db` (or `@rivetkit/sqlite-vfs`) is the explicit opt-in for SQLite. Do not lazily load SQLite from `rivetkit/db`; it may be imported eagerly inside that entrypoint.
- Core drivers must remain SQLite-agnostic. Any SQLite-specific wiring belongs behind the `rivetkit/db` or `@rivetkit/sqlite-vfs` boundary.

## Context Types Sync

The `*ContextOf` types exported from `packages/rivetkit/src/actor/contexts/index.ts` are documented in two places that must be kept in sync when adding, removing, or renaming context types:

- `website/src/content/docs/actors/types.mdx` — public docs page
- `website/src/content/docs/actors/index.mdx` — crash course (Context Types section)

## Raw KV Limits

When working with raw actor KV, always enforce engine limits:

- Max key size: 2048 bytes.
- Max batch payload size (`kv put`): 976 KiB total across keys + values.
- Max entries per batch (`kv put`): 128 key-value pairs.
- Max total actor KV storage: 10 GiB.

All raw KV operations must be designed to handle these constraints. If an operation can exceed a per-request limit, split/chunk it into multiple KV operations instead of relying on engine-side failures.

The total actor KV storage limit (10 GiB) cannot be worked around by chunking. Any KV operation can still fail due to storage limits. Always handle the error path cleanly and fail closed by default so the error surfaces to the user. Do not silently swallow, truncate, or ignore KV write failures.

When changing KV, queue, workflow persistence, SQLite-over-KV, or any limit-related actor behavior, update `website/src/content/docs/actors/limits.mdx` in the same change so docs stay in sync with effective hard and soft limits.

## Workflow Context Actor Access Guards

In `ActorWorkflowContext` (`packages/rivetkit/src/workflow/context.ts`), all side-effectful `#runCtx` access must be guarded by `#ensureActorAccess` so that side effects only run inside workflow steps and are not replayed outside of them. Read-only properties (e.g., `actorId`, `log`) do not need guards. When adding new methods or properties to the workflow context that delegate to `#runCtx`, apply the guard if the operation has side effects.
