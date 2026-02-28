# rivetkit-typescript/CLAUDE.md

## Tree-Shaking Boundaries

- Do not import `@rivetkit/workflow-engine` outside the `rivetkit/workflow` entrypoint so it remains tree-shakeable.
- Do not import SQLite VFS or `@rivetkit/sqlite` outside the `rivetkit/db` (or `@rivetkit/sqlite-vfs`) entrypoint so SQLite support remains tree-shakeable.
- Importing `rivetkit/db` (or `@rivetkit/sqlite-vfs`) is the explicit opt-in for SQLite. Do not lazily load SQLite from `rivetkit/db`; it may be imported eagerly inside that entrypoint.
- Core drivers must remain SQLite-agnostic. Any SQLite-specific wiring belongs behind the `rivetkit/db` or `@rivetkit/sqlite-vfs` boundary.

## Context Types Sync

- Keep the `*ContextOf` types exported from `packages/rivetkit/src/actor/contexts/index.ts` in sync with the two docs locations below when adding, removing, or renaming context types.

- `website/src/content/docs/actors/types.mdx` — public docs page
- `website/src/content/docs/actors/index.mdx` — crash course (Context Types section)

## Raw KV Limits

- Always enforce engine limits when working with raw actor KV.

- Max key size: 2048 bytes.
- Max batch payload size (`kv put`): 976 KiB total across keys + values.
- Max entries per batch (`kv put`): 128 key-value pairs.
- Max total actor KV storage: 10 GiB.

- Design raw KV operations to handle these constraints, and split operations into multiple requests if a per-request limit can be exceeded.
- Treat the total actor KV storage limit (10 GiB) as a hard limit, and fail closed with explicit errors instead of swallowing, truncating, or ignoring KV write failures.
- Update `website/src/content/docs/actors/limits.mdx` in the same change when KV, queue, workflow persistence, SQLite-over-KV, or any limit-related actor behavior changes.

## Workflow Context Actor Access Guards

- Guard all side-effectful `#runCtx` access in `ActorWorkflowContext` (`packages/rivetkit/src/workflow/context.ts`) with `#ensureActorAccess`; only read-only properties (for example `actorId` and `log`) are exempt.
- Apply `#ensureActorAccess` to any new workflow-context method or property that delegates to `#runCtx` and has side effects.

## Dynamic Actors Architecture Doc

- Reference `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md` when working on dynamic actor behavior, bridge contracts, isolate lifecycle, or runtime sandbox wiring.
- Keep `docs-internal/rivetkit-typescript/DYNAMIC_ACTORS_ARCHITECTURE.md` up to date in the same change whenever dynamic actor architecture, lifecycle, bridge payloads, security behavior, or temporary compatibility paths change.
