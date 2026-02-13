# rivetkit-typescript/CLAUDE.md

## Tree-Shaking Boundaries

- Do not import `@rivetkit/workflow-engine` outside the `rivetkit/workflow` entrypoint so it remains tree-shakeable.
- Do not import SQLite VFS or `wa-sqlite` outside the `rivetkit/db` (or `@rivetkit/sqlite-vfs`) entrypoint so SQLite support remains tree-shakeable.
- Importing `rivetkit/db` (or `@rivetkit/sqlite-vfs`) is the explicit opt-in for SQLite. Do not lazily load SQLite from `rivetkit/db`; it may be imported eagerly inside that entrypoint.
- Core drivers must remain SQLite-agnostic. Any SQLite-specific wiring belongs behind the `rivetkit/db` or `@rivetkit/sqlite-vfs` boundary.

## Context Types Sync

The `*ContextOf` types exported from `packages/rivetkit/src/actor/contexts/index.ts` are documented in two places that must be kept in sync when adding, removing, or renaming context types:

- `website/src/content/docs/actors/types.mdx` — public docs page
- `website/src/content/docs/actors/index.mdx` — crash course (Context Types section)
